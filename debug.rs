use nattydate::{parse_iso};

fn main() {
    println!("{:?}", parse_iso("2026-03-18T08:00:00Z"));
}
