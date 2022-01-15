use std::io::Write;

fn main() {
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
    let mut wordlist = file.lines().collect::<Vec<_>>();
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
        println!("Try: {}", tried);
        print!("Result: ");
        std::io::stdout().flush().unwrap();
        let mut result = String::new();
        std::io::stdin().read_line(&mut result).unwrap();
        if result.to_uppercase().trim() == "ZZZZZ" {
            println!("Yay!");
            break;
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
}
