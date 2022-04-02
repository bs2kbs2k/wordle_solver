use std::vec;

use chrono::TimeZone;

use serde_derive::Deserialize;

#[derive(Debug, Deserialize)]
struct Response {
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
    result: String,
}

#[async_std::main]
async fn main() -> anyhow::Result<()> {
    let client = surf::Client::new();
    let token = std::env::var("TOKEN").expect("TOKEN not set");
    let response = client
        .get("https://htsea.qixils.dev/api/wordle/info")
        .header("Cookie", format!("webToken={}", token))
        .recv_json::<State>()
        .await
        .map_err(|err| anyhow::anyhow!("{}", err))?;
    println!("Logged in! Info: {:?}", response);
    if let Some(cooldown) = response.cooldown {
        async_std::task::sleep(
            (chrono::Utc.timestamp(cooldown as i64 + 5, 0) - chrono::Utc::now())
                .to_std()
                .unwrap(),
        )
        .await;
    }
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
        if wordlist.len() < 5 {
            println!("{:?}", wordlist);
        }
        let mut wordlist_copy = orig_wordlist.clone();
        let mut cache = std::collections::HashMap::new();
        if wordlist == orig_wordlist {
            wordlist_copy = vec!["arose"];
        } else {
            wordlist_copy.sort_unstable_by(|a, b| {
                let mut a_score = 0.;
                if cache.contains_key(&a.to_string()) {
                    a_score = *cache.get(&a.to_string()).unwrap();
                } else {
                    let a_poss = get_all_possiblities(&nowhere, known, &somewhere, &not_here, a);

                    for a_hint in a_poss.iter() {
                        let mut nowhere = nowhere.clone();
                        let mut known = known.clone();
                        let mut somewhere = somewhere.clone();
                        let mut not_here = not_here.clone();
                        a_score += filter_wordlist(
                            a,
                            a_hint.clone(),
                            &mut nowhere,
                            &mut somewhere,
                            &mut not_here,
                            &mut known,
                            wordlist.clone(),
                        )
                        .len() as f64;
                    }
                    a_score /= a_poss.len() as f64;
                    cache.insert(a.to_string(), a_score);
                }
                let mut b_score = 0.;
                if cache.contains_key(&b.to_string()) {
                    b_score = *cache.get(&b.to_string()).unwrap();
                } else {
                    let b_poss = get_all_possiblities(&nowhere, known, &somewhere, &not_here, b);

                    for b_hint in b_poss.iter() {
                        let mut nowhere = nowhere.clone();
                        let mut known = known.clone();
                        let mut somewhere = somewhere.clone();
                        let mut not_here = not_here.clone();
                        b_score += filter_wordlist(
                            b,
                            b_hint.clone(),
                            &mut nowhere,
                            &mut somewhere,
                            &mut not_here,
                            &mut known,
                            wordlist.clone(),
                        )
                        .len() as f64;
                    }
                    b_score /= b_poss.len() as f64;
                    cache.insert(b.to_string(), b_score);
                }
                b_score.partial_cmp(&a_score).unwrap()
            });
        }
        let tried;
        if wordlist.len() == 1 {
            tried = wordlist.pop().unwrap();
        } else {
            tried = wordlist_copy.pop().expect("dafuq");
        }
        println!("Trying: {}", tried);
        let response = client
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
            async_std::task::sleep(
                (chrono::Utc.timestamp(response.new_state.cooldown.unwrap() as i64 + 5, 0)
                    - chrono::Utc::now())
                .to_std()
                .unwrap(),
            )
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
        wordlist = filter_wordlist(
            tried,
            result,
            &mut nowhere,
            &mut somewhere,
            &mut not_here,
            &mut known,
            wordlist,
        );
    }
}

fn filter_wordlist<'a>(
    tried: &str,
    result: String,
    nowhere: &mut std::collections::HashSet<char>,
    somewhere: &mut std::collections::HashSet<char>,
    not_here: &mut [std::collections::HashSet<char>; 5],
    known: &mut [Option<char>; 5],
    wordlist: Vec<&'a str>,
) -> Vec<&'a str> {
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
    wordlist
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

fn get_all_possiblities(
    nowhere: &std::collections::HashSet<char>,
    known: [Option<char>; 5],
    somewhere: &std::collections::HashSet<char>,
    not_here: &[std::collections::HashSet<char>; 5],
    guess: &str,
) -> Vec<String> {
    let mut possibilities = vec!["".to_string()];
    for index in 0..5 {
        let mut new_possiblities = Vec::new();
        for possiblity in possibilities.iter() {
            if known[index].is_some() {
                if known[index] == guess.chars().nth(index) {
                    new_possiblities.push(possiblity.clone() + "Z");
                } else if not_here[index].contains(&guess.chars().nth(index).unwrap()) {
                    new_possiblities.push(possiblity.clone() + "X");
                } else {
                    new_possiblities.push(possiblity.clone() + "X");
                    new_possiblities.push(possiblity.clone() + "Y");
                }
            } else if nowhere.contains(&guess.chars().nth(index).unwrap()) {
                new_possiblities.push(possiblity.clone() + "X");
            } else if somewhere.contains(&guess.chars().nth(index).unwrap()) {
                new_possiblities.push(possiblity.clone() + "Y");
            } else {
                new_possiblities.push(possiblity.clone() + "X");
                new_possiblities.push(possiblity.clone() + "Y");
            }
        }
        possibilities = new_possiblities;
    }
    possibilities
}
