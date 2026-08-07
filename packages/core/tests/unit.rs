use grove_core::{parse_fixture, parse_strict, serialize, serialize_body, verify_checksum};

fn wrap(body: &str) -> String {
    format!(
        "@grove 1\n# AUTO-GENERATED. Do not edit. Use `grove` CLI.\n# checksum: sha256:{}\n{}",
        "0".repeat(64),
        body
    )
}

#[test]
fn escaping_roundtrip() {
    let body = "q Q-01 status=open cynefin=clear t_created=2026-01-01T00:00:00Z t_updated=2026-01-01T00:00:00Z \"quote \\\" backslash \\\\ newline \\n tab\ttail\"\n";
    let st = parse_strict(&wrap(body)).expect("parse");
    assert_eq!(
        st.nodes["Q-01"].title,
        "quote \" backslash \\ newline \n tab\ttail"
    );
    assert_eq!(serialize_body(&st), body);
}

#[test]
fn bareword_vs_qstring_choice() {
    let body = "g G-01 status=unverified fitness=1/1 t_created=2026-01-01T00:00:00Z t_updated=2026-01-01T00:00:00Z \"A\"\n\ng G-02 status=unverified fitness=\"1 of 1\" t_created=2026-01-01T00:00:00Z t_updated=2026-01-01T00:00:00Z \"B\"\n\ne G-01 blocks G-02\n";
    let st = parse_strict(&wrap(body)).expect("parse");
    let out = serialize_body(&st);
    assert!(out.contains("fitness=1/1 "), "bare word stays bare: {out}");
    assert!(
        out.contains("fitness=\"1 of 1\" "),
        "value with space stays quoted: {out}"
    );
    assert!(
        out.contains("e G-01 blocks G-02 t_created=\"\"\n"),
        "missing t_created serializes as empty qstring: {out}"
    );
    assert_eq!(
        out,
        body.replace("e G-01 blocks G-02\n", "e G-01 blocks G-02 t_created=\"\"\n")
    );
}

const SHUFFLED: &str = r#"@grove 1
# AUTO-GENERATED. Do not edit. Use `grove` CLI.
# checksum: sha256:0000000000000000000000000000000000000000000000000000000000000000

# hand note

a A-01 t_updated=2026-01-01T00:00:00Z status=present t_created=2026-01-01T00:00:00Z "Area"
  surface: src/a.jl, src/b.jl

e W-01 blocks W-02 t_created=2026-01-01T00:00:00Z
e Q-01 asks W-01 t_created=2026-01-01T00:00:00Z
e B-01 tests Q-01 t_created=2026-01-01T00:00:00Z

w W-02 type=bug status=proposed cynefin=clear t_updated=2026-01-01T00:00:00Z t_created=2026-01-01T00:00:00Z "Second work"

g G-02 status=partial fitness=1/2 t_updated=2026-01-01T00:00:00Z t_created=2026-01-01T00:00:00Z "Second goal"
  area: A-01

y Y-01 status=active t_created=2026-01-01T00:00:00Z t_updated=2026-01-01T00:00:00Z "Disc"
  surface: src/a.jl
  tags: zeta, alpha
  why:
    | second prose line
    | first? no, insertion order kept

w W-01 type=feature status=ready cynefin=clear t_created=2026-01-01T00:00:00Z t_updated=2026-01-01T00:00:00Z "First work"
  why:
    | why line
  ac:
    | ac one
    | ac two
  fitness: G-02=-1, G-01=+2
  goals: G-01, G-02
  surface: src/x.jl

g G-01 status=unverified t_created=2026-01-01T00:00:00Z t_updated=2026-01-01T00:00:00Z "First goal"
  notes:
    | note line
  tags: x, y
  area: A-01

q Q-01 status=open cynefin=clear t_created=2026-01-01T00:00:00Z t_updated=2026-01-01T00:00:00Z "Quest"

b B-01 status=testing cynefin=clear t_created=2026-01-01T00:00:00Z t_updated=2026-01-01T00:00:00Z "Bet"

d D-01 status=proposed t_created=2026-01-01T00:00:00Z t_updated=2026-01-01T00:00:00Z "Dec"

t T-01 status=open t_created=2026-01-01T00:00:00Z t_updated=2026-01-01T00:00:00Z "Theme"
  notes:
    | t note
"#;

const CANONICAL_BODY: &str = r#"g G-01 status=unverified t_created=2026-01-01T00:00:00Z t_updated=2026-01-01T00:00:00Z "First goal"
  area: A-01
  tags: x, y
  notes:
    | note line

g G-02 status=partial fitness=1/2 t_created=2026-01-01T00:00:00Z t_updated=2026-01-01T00:00:00Z "Second goal"
  area: A-01

w W-01 type=feature status=ready cynefin=clear t_created=2026-01-01T00:00:00Z t_updated=2026-01-01T00:00:00Z "First work"
  goals: G-01, G-02
  fitness: G-01=+2, G-02=-1
  surface: src/x.jl
  ac:
    | ac one
    | ac two
  why:
    | why line

w W-02 type=bug status=proposed cynefin=clear t_created=2026-01-01T00:00:00Z t_updated=2026-01-01T00:00:00Z "Second work"

d D-01 status=proposed t_created=2026-01-01T00:00:00Z t_updated=2026-01-01T00:00:00Z "Dec"

q Q-01 status=open cynefin=clear t_created=2026-01-01T00:00:00Z t_updated=2026-01-01T00:00:00Z "Quest"

b B-01 status=testing cynefin=clear t_created=2026-01-01T00:00:00Z t_updated=2026-01-01T00:00:00Z "Bet"

t T-01 status=open t_created=2026-01-01T00:00:00Z t_updated=2026-01-01T00:00:00Z "Theme"
  notes:
    | t note

y Y-01 status=active t_created=2026-01-01T00:00:00Z t_updated=2026-01-01T00:00:00Z "Disc"
  tags: zeta, alpha
  surface: src/a.jl
  why:
    | second prose line
    | first? no, insertion order kept

a A-01 status=present t_created=2026-01-01T00:00:00Z t_updated=2026-01-01T00:00:00Z "Area"
  surface: src/a.jl, src/b.jl

e B-01 tests Q-01 t_created=2026-01-01T00:00:00Z
e Q-01 asks W-01 t_created=2026-01-01T00:00:00Z
e W-01 blocks W-02 t_created=2026-01-01T00:00:00Z
"#;

#[test]
fn canonical_ordering_matches_julia() {
    let st = parse_strict(SHUFFLED).expect("parse shuffled");
    assert_eq!(serialize_body(&st), CANONICAL_BODY);
    let st = parse_strict(&wrap(CANONICAL_BODY)).expect("parse canonical");
    assert_eq!(serialize_body(&st), CANONICAL_BODY);
}

#[test]
fn checksum_tamper_detection() {
    let body = "w W-01 type=feature status=ready cynefin=clear t_created=2026-01-01T00:00:00Z t_updated=2026-01-01T00:00:00Z \"Work\"\n  ac:\n    | line one\n";
    let st = parse_strict(&wrap(body)).expect("parse");
    let full = serialize(&st);
    assert!(verify_checksum(&full), "fresh serialization verifies");

    let tampered_body = full.replacen("status=ready", "status=readx", 1);
    assert_ne!(tampered_body, full);
    assert!(!verify_checksum(&tampered_body), "body tamper detected");

    const PREFIX: &str = "# checksum: sha256:";
    let at = full.find(PREFIX).unwrap() + PREFIX.len();
    let first = full.as_bytes()[at] as char;
    let other = if first == '0' { '1' } else { '0' };
    let tampered_hex = format!("{}{}{}", &full[..at], other, &full[at + 1..]);
    assert!(!verify_checksum(&tampered_hex), "checksum tamper detected");
}

#[test]
fn crlf_handled_like_julia() {
    let body = "w W-01 type=feature status=ready cynefin=clear t_created=2026-01-01T00:00:00Z t_updated=2026-01-01T00:00:00Z \"Work\"\n  ac:\n    | line one\n";
    let st = parse_strict(&wrap(body)).expect("parse");
    let full = serialize(&st);

    let crlf = full.replace('\n', "\r\n");
    assert!(verify_checksum(&crlf), "crlf file verifies after normalization");
    let st2 = parse_strict(&crlf).expect("crlf parse");
    assert_eq!(serialize(&st2), full, "crlf parse equals lf parse");

    let lone_cr = full.replace('\n', "\r");
    assert!(
        parse_fixture(&lone_cr).is_err(),
        "lone CR is not a line ending, same as julia"
    );
}
