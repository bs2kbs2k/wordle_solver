use std::io::Write;

use serde_derive::Deserialize;

#[derive(Debug, Deserialize)]
struct Response {
    success: bool,
    new_state: State,
}

#[derive(Debug, Deserialize)]
struct State {
    guesses: Vec<Guess>,
    cooldown: Option<f64>,
    diamonds: f64,
}

#[derive(Debug, Deserialize)]
struct Guess {
    word: String,
    result: String,
}

#[async_std::main]
async fn main() -> anyhow::Result<()> {
    let client = surf::Client::new();
    let token = std::env::var("TOKEN").expect("TOKEN not set");
    let file = std::env::args()
        .nth(1)
        .map(|path| std::fs::read_to_string(path).ok())
        .flatten()
        .unwrap_or_else(|| {
            println!("Using default wordlist as none was specified");
            include_str!("default.txt").to_string()
        });
    let freq_map = file.to_lowercase().replace("\n", "").chars().fold(
        std::collections::HashMap::new(),
        |mut acc, elem| {
            match acc.get_mut(&elem) {
                Some(cnt) => {
                    *cnt += 1;
                }
                None => {
                    acc.insert(elem, 1);
                }
            }
            acc
        },
    );
    let orig_wordlist = file.lines().collect::<Vec<_>>();
    let mut wordlist = orig_wordlist.clone();
    println!("Loaded {} words", wordlist.len());
    let mut freq = freq_map.keys().copied().collect::<Vec<_>>();
    freq.sort_by(|a, b| freq_map.get(b).unwrap().cmp(freq_map.get(a).unwrap()));
    let freq = freq.into_iter().collect::<String>();
    let mut nowhere: std::collections::HashSet<char> = std::collections::HashSet::new();
    let mut somewhere: std::collections::HashSet<char> = std::collections::HashSet::new();
    let mut not_here: [std::collections::HashSet<char>; 5] = [
        std::collections::HashSet::new(),
        std::collections::HashSet::new(),
        std::collections::HashSet::new(),
        std::collections::HashSet::new(),
        std::collections::HashSet::new(),
    ];
    let mut known: [Option<char>; 5] = [None; 5];
    println!(
        r#"Wordle Solver
Type word into wordle when prompted
Type result into program in this format:
    Gray  : X
    Yellow: Y
    Green : Z"#
    );
    loop {
        println!("There are {} possible solutions", wordlist.len());
        wordlist.sort_unstable_by(|a, b| {
            let mut a: Vec<char> = a.to_lowercase().chars().collect();
            let mut b: Vec<char> = b.to_lowercase().chars().collect();
            a.sort_unstable();
            b.sort_unstable();
            let uniq_a = a
                .iter()
                .fold(std::collections::HashSet::new(), |mut acc, elem| {
                    acc.insert(*elem);
                    acc
                });
            let uniq_b = b
                .iter()
                .fold(std::collections::HashSet::new(), |mut acc, elem| {
                    acc.insert(*elem);
                    acc
                });
            if uniq_a.len() == uniq_b.len() {
                let mut commonness = (0, 0);
                for letter in a {
                    if let Some(idx) = freq.find(letter) {
                        commonness.0 += idx
                    }
                }
                for letter in b {
                    if let Some(idx) = freq.find(letter) {
                        commonness.1 += idx
                    }
                }
                return commonness.1.cmp(&commonness.0);
            }
            uniq_a.len().cmp(&uniq_b.len())
        });
        let tried = wordlist.pop().expect("No possible solutions?");
        println!("Trying: {}", tried);
        let mut response = client
            .post(format!(
                "https://htsea.qixils.dev/api/wordle/guess?guess={}",
                tried
            ))
            .header("Cookie", format!("webToken={}", token))
            .recv_json::<Response>()
            .await
            .map_err(|err| anyhow::anyhow!("{}", err))?;
        let result = response
            .new_state
            .guesses
            .last()
            .unwrap()
            .result
            .clone()
            .replace("y", "Y")
            .replace("g", "Z")
            .replace("x", "X");
        println!("Result: {}", result);
        println!("Diamonds: {}", response.new_state.diamonds);
        println!("Response: {:?}", response);
        if response.new_state.cooldown.is_some() {
            println!("Next!");
            async_std::task::sleep(std::time::Duration::from_secs_f64(
                response.new_state.cooldown.unwrap() / 10000000f64 + 10f64,
            ))
            .await;
            nowhere.clear();
            somewhere.clear();
            not_here = [
                std::collections::HashSet::new(),
                std::collections::HashSet::new(),
                std::collections::HashSet::new(),
                std::collections::HashSet::new(),
                std::collections::HashSet::new(),
            ];
            known = [None, None, None, None, None];
            wordlist = orig_wordlist.clone();
            continue;
        }
        for (index, letter) in result.to_uppercase().trim().char_indices() {
            match letter {
                'X' => {
                    nowhere.insert(tried.chars().nth(index).unwrap());
                }
                'Y' => {
                    somewhere.insert(tried.chars().nth(index).unwrap());
                    not_here[index].insert(tried.chars().nth(index).unwrap());
                    nowhere.remove(&tried.chars().nth(index).unwrap());
                }
                'Z' => {
                    known[index] = Some(tried.chars().nth(index).unwrap());
                    nowhere.remove(&tried.chars().nth(index).unwrap());
                }
                _ => panic!("Invalic Character"),
            }
        }
        wordlist = wordlist
            .into_iter()
            .filter(|elem| {
                for (index, letter) in elem.char_indices() {
                    if let Some(char) = known[index] {
                        if char != letter {
                            return false;
                        }
                    }
                    if nowhere.contains(&letter) {
                        return false;
                    }
                    if not_here[index].contains(&letter) {
                        return false;
                    }
                }
                for letter in somewhere.iter() {
                    if !elem.contains(*letter) {
                        return false;
                    }
                }
                true
            })
            .collect()
    }
    Ok(())
}
