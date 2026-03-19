use nattydate::{ParseConfig, tokenize_and_classify};

fn main() {
    let mut config = ParseConfig::default();
    config.debug = true;
    let tokens = tokenize_and_classify("7/6/26", &config);
    println!("{:#?}", tokens);
}
