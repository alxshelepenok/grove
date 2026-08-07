mod common;
#[allow(unused_imports)]
use common::*;
use grove_core::*;

fn wrap(body: &str) -> String {
    format!(
        "@grove 1\n# AUTO-GENERATED. Do not edit. Use `grove` CLI.\n# checksum: sha256:{}\n{}",
        "0".repeat(64),
        body
    )
}

#[test]
fn parse_id_numeric_parses_family_and_suffix() {
    assert_eq!(parse_id_numeric("W-01"), Ok(('W', 1)));
    assert_eq!(parse_id_numeric("G-12"), Ok(('G', 12)));
    assert_eq!(parse_id_numeric("A-007"), Ok(('A', 7)));
    assert_eq!(parse_id_numeric(" W-03 "), Ok(('W', 3)));
}

#[test]
fn parse_id_numeric_rejects_malformed_ids() {
    assert_eq!(
        parse_id_numeric("W-00"),
        Err("malformed id: W-00".to_string())
    );
    assert_eq!(
        parse_id_numeric("w-01"),
        Err("malformed id: w-01".to_string())
    );
    assert_eq!(
        parse_id_numeric("W01"),
        Err("malformed id: W01".to_string())
    );
    assert_eq!(
        parse_id_numeric("W-"),
        Err("malformed id: W-".to_string())
    );
    assert_eq!(parse_id_numeric(""), Err("malformed id: ".to_string()));
    assert_eq!(
        parse_id_numeric("W-01x"),
        Err("malformed id: W-01x".to_string())
    );
}

#[test]
fn format_allocated_id_pads_and_grows() {
    assert_eq!(format_allocated_id('W', 1, 2), "W-01");
    assert_eq!(format_allocated_id('W', 123, 2), "W-123");
    assert_eq!(format_allocated_id('G', 5, 4), "G-0005");
    assert_eq!(format_allocated_id('W', 0, 2), "W-00");
}

#[test]
fn next_id_legacy_sequence_starts_at_one() {
    let mut st = State::default();
    assert_eq!(next_id(&mut st, Kind::W), "W-01");
    assert_eq!(next_id(&mut st, Kind::W), "W-02");
}

#[test]
fn next_id_uses_recorded_high_water_mark() {
    let mut st = State::default();
    record_id(&mut st, "W-07");
    assert_eq!(next_id(&mut st, Kind::W), "W-08");
}

#[test]
fn next_id_honors_stride_offset_and_pad() {
    let mut st = State::default();
    st.id_stride = 5;
    st.id_offset = 10;
    st.id_pad_width = 3;
    assert_eq!(next_id(&mut st, Kind::W), "W-010");
    assert_eq!(next_id(&mut st, Kind::W), "W-015");
}

#[test]
fn record_id_keeps_max_only() {
    let mut st = State::default();
    record_id(&mut st, "W-09");
    record_id(&mut st, "W-03");
    assert_eq!(next_id(&mut st, Kind::W), "W-10");
}

#[test]
fn reconcile_counters_scans_nodes_and_edges() {
    let mut st = State::default();
    put(&mut st, work("W-03", "feature", "ready", "clear"));
    put(&mut st, work("W-04", "feature", "ready", "clear"));
    put(&mut st, plain(Kind::G, "G-02", "unverified"));
    edge(&mut st, "W-03", "blocks", "W-04");
    st.counters.insert('W', 99);
    reconcile_counters(&mut st);
    assert_eq!(st.counters.get(&'W'), Some(&4));
    assert_eq!(st.counters.get(&'G'), Some(&2));
    assert_eq!(next_id(&mut st, Kind::W), "W-05");
}

#[test]
fn parse_updates_counters_from_node_ids() {
    let text = wrap("w W-07 type=feature status=ready cynefin=clear \"t\"\n");
    let mut st = parse_fixture(&text).expect("parse");
    assert_eq!(st.counters.get(&'W'), Some(&7));
    assert_eq!(next_id(&mut st, Kind::W), "W-08");
}

#[test]
fn parse_id_meta_line_sets_stride_offset_pad() {
    let text = wrap("# @grove-id stride=2 offset=5 pad=3\nw W-01 type=feature status=ready cynefin=clear \"t\"\n");
    let mut st = parse_fixture(&text).expect("parse");
    assert_eq!(st.id_stride, 2);
    assert_eq!(st.id_offset, 5);
    assert_eq!(st.id_pad_width, 3);
    assert_eq!(next_id(&mut st, Kind::W), "W-003");
}
