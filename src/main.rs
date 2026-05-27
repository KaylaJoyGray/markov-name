use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader};
use wyrand::WyRand;

/// A categorical distribution with additive smoothing.
#[derive(Clone, Debug)]
struct Categorical {
    counts: HashMap<char, f64>,
    total: f64,
}

impl Categorical {
    fn new(alphabet: &[char], prior: f64) -> Self {
        let mut counts = HashMap::new();
        let mut total = 0.0;

        for &c in alphabet {
            counts.insert(c, prior);
            total += prior;
        }

        Self { counts, total }
    }

    fn observe(&mut self, event: char, count: f64) {
        *self.counts.entry(event).or_insert(0.0) += count;
        self.total += count;
    }

    fn sample(&self, rng: &mut WyRand) -> char {
        let mut r = rng.rand() as f64 % self.total;

        for (&ch, &weight) in &self.counts {
            if r <= weight {
                return ch;
            }
            r -= weight;
        }

        // fallback (floating point edge case)
        *self.counts.keys().next().unwrap()
    }
}

/// High-order Markov model with Katz-style backoff.
pub struct MarkovModel {
    order: usize,
    alphabet: Vec<char>,
    tables: Vec<HashMap<Vec<char>, Categorical>>, // order-specific models
    prior: f64,
    start: char,
    end: char,
    dataset: HashSet<String,>
}

impl MarkovModel {
    pub fn new(alphabet: Vec<char>, order: usize, prior: f64, dataset: HashSet<String>) -> Self {
        let mut tables = Vec::new();

        for _ in 0..=order {
            tables.push(HashMap::new());
        }

        Self {
            order,
            alphabet,
            tables,
            prior,
            start: '^',
            end: '$',
            dataset,
        }
    }

    fn get_cat(&mut self, order: usize, context: &[char]) -> &mut Categorical {
        let table = &mut self.tables[order];

        table
            .entry(context.to_vec())
            .or_insert_with(|| {
                let mut alphabet = self.alphabet.clone();
                alphabet.push(self.end);
                Categorical::new(&alphabet, self.prior)
            })
    }

    fn observe_sequence(&mut self, seq: &str) {
        let mut chars: Vec<char> = Vec::new();
        chars.push(self.start);
        chars.push(self.start);
        chars.extend(seq.chars());
        chars.push(self.end);

        for i in 2..chars.len() {
            let target = chars[i];

            for k in 0..=self.order {
                if i < k {
                    continue;
                }

                let context_start = i - k;
                let context = &chars[context_start..i];

                let cat = self.get_cat(k, context);
                cat.observe(target, 1.0);
            }
        }
    }

    fn backoff_context(&self, context: &[char], k: usize) -> Vec<char> {
        let mut ctx = context.to_vec();

        if ctx.len() > k {
            ctx = ctx[ctx.len() - k..].to_vec();
        } else if ctx.len() < k {
            let mut padded = vec![self.start; k - ctx.len()];
            padded.extend(ctx);
            ctx = padded;
        }

        ctx
    }

    fn sample_from_context(&self, rng: &mut WyRand, context: &[char]) -> char {
        for k in (0..=self.order).rev() {
            let ctx = self.backoff_context(context, k);

            if let Some(table) = self.tables[k].get(&ctx) {
                return table.sample(rng);
            }
        }

        self.end
    }

    pub fn generate(&self, rng: &mut WyRand) -> String {
        let mut result: String = String::new();
        for _ in 0..10 {
            let mut characters = vec![self.start, self.start];

            loop {
                let next = self.sample_from_context(rng, &characters);
                if next == self.end {
                    break;
                }
                characters.push(next);
            }

            result = characters.into_iter().skip(2).collect();
            if !self.dataset.contains(&result) {
                break
            }
        }
        result
    }
}

pub struct NameGenerator {
    model: MarkovModel,
}

impl NameGenerator {
    pub fn new<I>(name_list: I, order: usize, prior: f64) -> Self
    where
        I: IntoIterator<Item = String>,
    {
        let mut names: HashSet<String> = HashSet::new();
        let mut support: HashSet<char> = HashSet::new();

        for name in name_list {
            let name = name.trim();

            if !name.is_empty() {
                names.insert(name.to_string());

                for c in name.chars() {
                    support.insert(c);
                }
            }
        }

        let dataset = names.clone();

        let alphabet: Vec<char> = support.into_iter().collect();

        let mut model = MarkovModel::new(alphabet, order, prior, dataset);

        for name in names {
            model.observe_sequence(&name);
        }

        Self { model }
    }

    pub fn generate(&self, rng: &mut WyRand) -> String {
        self.model.generate(rng)
    }
}

fn main() {
    let mut names = Vec::new();

    let Ok(file) = File::open("./data/places1.txt") else {
        return;
    };

    let reader = BufReader::new(file);
    for line in reader.lines() {
        if let Ok(line) = line {
            names.push(line);
        }
    }

    // let Ok(file) = File::open("./data/aztec-male.txt") else {
    //     return;
    // };
    //
    // let reader = BufReader::new(file);
    // for line in reader.lines() {
    //     if let Ok(line) = line {
    //         names.push(line);
    //     }
    // }

    // let Ok(file) = File::open("./data/egyptian-male.txt") else {
    //     return;
    // };
    //
    // let reader = BufReader::new(file);
    // for line in reader.lines() {
    //     if let Ok(line) = line {
    //         names.push(line);
    //     }
    // }
    //
    // let Ok(file) = File::open("./data/egyptian-female.txt") else {
    //     return;
    // };
    //
    // let reader = BufReader::new(file);
    // for line in reader.lines() {
    //     if let Ok(line) = line {
    //         names.push(line);
    //     }
    // }


    let generator = NameGenerator::new(names, 3, 0.001, /* char */);

    let mut rng = WyRand::new(123);

    for _ in 0..10 {
        println!("{}", generator.generate(&mut rng));
    }
}