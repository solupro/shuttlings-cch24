use rand::distributions::Alphanumeric;
use rand::Rng;

pub mod day2;
pub mod day5;
pub mod day9;
pub mod day12;

pub mod day16;
pub mod day19;
pub mod day23;


pub fn generate_token(n :usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(n)
        .map(char::from)
        .collect()
}