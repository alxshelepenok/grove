use grove_core::{emit_jval, julia_string_hash, parse_json, JVal, JuliaDict, Json};

fn keys(d: &JuliaDict) -> Vec<String> {
    d.iter_pairs().map(|(k, _)| k.clone()).collect()
}

#[test]
fn julia_string_hash_matches_ground_truth() {
    let cases: &[(&str, u64)] = &[
        ("A-01", 9197219131717075166),
        ("G-01", 17227821226381323349),
        ("G-02", 7275990776660668974),
        ("G-03", 29833850370282953),
        ("W-01", 1876897830945138734),
        ("W-02", 1397171917063913118),
        ("Y-01", 11040328189078086937),
        ("Y-02", 13711983608097687752),
        ("Y-03", 18015910948805506704),
        ("T-01", 11387001563392791935),
        ("op", 395050573414855068),
        ("id", 15249407420556684696),
        ("v", 11689616896668953568),
        ("ts", 7651692633868875982),
        ("cmd", 1421058193481851804),
        ("inv", 9860482052842355839),
        ("from", 13752649140593157884),
        ("to", 11887867837879061051),
        ("label", 4009463771286888128),
        ("old", 12749674392548337209),
        ("new", 13888762393600028594),
        ("tags", 11625941304095277755),
        ("map", 15686231507568066132),
        ("lines", 18179553462010425928),
        ("line", 3938981271834545889),
        ("index", 10093910054565570990),
        ("value", 14376741705065669659),
        ("field", 6530121189847464409),
        ("wid", 18034889763399022836),
        ("gid", 7628174618687964822),
        ("had_key", 9075082891895283346),
        ("previous", 1340242119363127702),
        ("steps", 2872771548895680899),
        ("missing", 5842114294087241506),
        ("goal", 840027898295283178),
        ("empty", 4760104213995324430),
        ("dones", 1601983191911881665),
        ("overflows", 7683409424892852037),
        ("invalidated", 6681722138606988277),
        ("tw", 14888971231030367244),
        ("overflow_counts", 17392715146398451869),
        ("old_status", 2951689508549640048),
        ("old_w_status", 1018453099761146052),
        ("goal_statuses", 4945704143635692988),
        ("had_session_before", 11442367392028345764),
        ("had_session_at_before", 6706815343447500478),
        ("old_session", 6658866169361020126),
        ("old_session_at", 3952725019794741692),
        ("had_before", 17261753696973081385),
        ("t_created", 4306920455067451623),
        ("glossary_changed", 11656235741206152579),
        ("fitness_target", 1151472378826096232),
    ];
    for (s, want) in cases {
        assert_eq!(julia_string_hash(s), *want, "hash mismatch for {s}");
    }
}

#[test]
fn dict_plain_insert_order_matches_julia() {
    let mut d = JuliaDict::new();
    d.insert("G-01".to_string(), JVal::Int(1));
    d.insert("G-02".to_string(), JVal::Int(2));
    d.insert("G-03".to_string(), JVal::Int(3));
    assert_eq!(keys(&d), vec!["G-01", "G-03", "G-02"]);

    let mut d2 = JuliaDict::new();
    for k in ["Y-01", "Y-02", "Y-03"] {
        d2.insert(k.to_string(), JVal::Null);
    }
    assert_eq!(keys(&d2), vec!["Y-03", "Y-02", "Y-01"]);
}

#[test]
fn dict_growth_order_matches_julia() {
    let mut d = JuliaDict::new();
    for i in 1..=15 {
        d.insert(format!("G-0{i}"), JVal::Int(i as i64));
    }
    let want = [
        "G-06", "G-03", "G-011", "G-012", "G-01", "G-09", "G-015", "G-013", "G-04", "G-05",
        "G-010", "G-07", "G-08", "G-02", "G-014",
    ];
    assert_eq!(keys(&d), want.iter().map(|s| s.to_string()).collect::<Vec<_>>());
}

#[test]
fn dict_varargs_ctor_order_matches_julia() {
    let d = JuliaDict::from_pairs(vec![
        ("op".to_string(), JVal::Str("x".to_string())),
        ("id".to_string(), JVal::Str("y".to_string())),
    ]);
    assert_eq!(keys(&d), vec!["id", "op"]);

    let w = JuliaDict::from_pairs(vec![
        ("v".to_string(), JVal::Int(1)),
        ("cmd".to_string(), JVal::Str("c".to_string())),
        ("ts".to_string(), JVal::Str("t".to_string())),
        ("inv".to_string(), JVal::Null),
    ]);
    assert_eq!(keys(&w), vec!["v", "cmd", "ts", "inv"]);
}

#[test]
fn dict_merge_and_copy_order_matches_julia() {
    let a = JuliaDict::from_pairs(vec![
        ("x".to_string(), JVal::Int(1)),
        ("y".to_string(), JVal::Int(2)),
    ]);
    let b = JuliaDict::from_pairs(vec![
        ("G-01".to_string(), JVal::Int(1)),
        ("G-02".to_string(), JVal::Int(2)),
        ("G-03".to_string(), JVal::Int(3)),
    ]);
    let mut m = JuliaDict::slot_copy(&a);
    m.merge_from(&b);
    assert_eq!(keys(&m), vec!["G-01", "x", "G-03", "G-02", "y"]);

    let c = JuliaDict::slot_copy(&b);
    assert_eq!(keys(&c), vec!["G-01", "G-03", "G-02"]);
}

#[test]
fn dict_update_keeps_slot() {
    let mut d = JuliaDict::new();
    d.insert("G-01".to_string(), JVal::Int(1));
    d.insert("G-02".to_string(), JVal::Int(2));
    d.insert("G-03".to_string(), JVal::Int(3));
    d.insert("G-02".to_string(), JVal::Int(9));
    assert_eq!(keys(&d), vec!["G-01", "G-03", "G-02"]);
    assert_eq!(d.len(), 3);
}

#[test]
fn emit_matches_json_jl_escaping() {
    assert_eq!(
        emit_jval(&JVal::Str("x\ty\nz\"q\"\\".to_string())),
        "\"x\\ty\\nz\\\"q\\\"\\\\\""
    );
    assert_eq!(
        emit_jval(&JVal::Str("\u{8}\u{c}\r\u{0}\u{7f}/".to_string())),
        "\"\\b\\f\\r\\u0000\\u007f/\""
    );
    assert_eq!(
        emit_jval(&JVal::Str("héllo—世界".to_string())),
        "\"héllo—世界\""
    );
    assert_eq!(emit_jval(&JVal::Int(-42)), "-42");
    assert_eq!(emit_jval(&JVal::Int(0)), "0");
    assert_eq!(emit_jval(&JVal::Bool(true)), "true");
    assert_eq!(emit_jval(&JVal::Bool(false)), "false");
    assert_eq!(emit_jval(&JVal::Null), "null");
    assert_eq!(
        emit_jval(&JVal::Arr(vec![JVal::Int(1), JVal::Str("a".to_string())])),
        "[1,\"a\"]"
    );
}

#[test]
fn parse_json_roundtrips_journal_values() {
    let v = parse_json(r#"{"a":[1,2.0,"x",null,true],"b":{"c":"d"},"e":"û\n"}"#).unwrap();
    let a = v.get("a").and_then(|x| x.as_arr()).unwrap();
    assert_eq!(a[0].as_i64(), Some(1));
    assert_eq!(a[1].as_i64(), Some(2));
    assert_eq!(a[2].as_str(), Some("x"));
    assert!(matches!(a[3], Json::Null));
    assert_eq!(a[4].as_bool(), Some(true));
    assert_eq!(
        v.get("b").and_then(|b| b.get("c")).and_then(|c| c.as_str()),
        Some("d")
    );
    assert_eq!(v.get("e").and_then(|e| e.as_str()), Some("û\n"));
    assert!(v.get("missing").is_none());
}

#[test]
fn parse_json_rejects_garbage() {
    assert!(parse_json("{").is_err());
    assert!(parse_json("{\"a\":}").is_err());
    assert!(parse_json("[1,]").is_err());
    assert!(parse_json("{\"a\":1} trailing").is_err());
}
