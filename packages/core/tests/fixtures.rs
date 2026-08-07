use grove_core::{checksum_of, parse, parse_strict, serialize, verify_checksum, ParseMode};
use std::fs;
use std::path::PathBuf;

fn fixture_text(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    fs::read_to_string(path).expect("fixture lock")
}

const FIXTURES: [&str; 3] = ["basic.lock", "discovery.lock", "archive.lock"];

#[test]
fn real_locks_strict_roundtrip_byte_identically() {
    for name in FIXTURES {
        let text = fixture_text(name);
        let state = parse_strict(&text).unwrap_or_else(|e| panic!("{name}: {e}"));
        let out = serialize(&state);
        assert_eq!(out, text, "{name}: strict roundtrip not byte-identical");
        let state = parse(&text, ParseMode::Fixture).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(serialize(&state), text, "{name}: fixture roundtrip differs");
    }
}

#[test]
fn real_locks_checksums_verify() {
    for name in FIXTURES {
        let text = fixture_text(name);
        assert!(verify_checksum(&text), "{name}: checksum does not verify");
    }
}

#[test]
fn archive_fixture_hash_proof() {
    let text = fixture_text("archive.lock");
    let state = parse_strict(&text).expect("strict parse");
    let out = serialize(&state);
    println!("sha256(fixture bytes)     = {}", checksum_of(&text));
    println!("sha256(reserialized bytes)= {}", checksum_of(&out));
    assert_eq!(checksum_of(&text), checksum_of(&out));
    assert!(!state.nodes.is_empty());
    assert!(state.nodes.values().any(|n| n.archived));
}
