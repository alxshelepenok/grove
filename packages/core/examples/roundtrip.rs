use std::io::Write;

fn main() {
    let path = std::env::args().nth(1).expect("usage: roundtrip <lock file>");
    let text = std::fs::read_to_string(&path).expect("read lock file");
    let state = grove_core::parse_strict(&text).expect("strict parse");
    std::io::stdout()
        .write_all(grove_core::serialize(&state).as_bytes())
        .expect("write stdout");
}
