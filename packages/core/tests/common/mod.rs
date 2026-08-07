#![allow(dead_code)]

use grove_core::{Edge, FieldValue, Kind, Node, State};
use std::collections::BTreeMap;

pub fn corpus_json(name: &str) -> serde_json::Value {
    let p = format!("../grove/conformance/corpus/{name}.json");
    let text = std::fs::read_to_string(&p)
        .or_else(|_| std::fs::read_to_string(format!("tests/fixtures/wave2b/{name}.json")))
        .unwrap_or_else(|_| panic!("corpus fixture not found: {name}"));
    serde_json::from_str(&text).unwrap()
}

pub fn step_field(sc: &serde_json::Value, i: usize, field: &str) -> String {
    sc["steps"][i][field].as_str().unwrap_or("").to_string()
}

pub fn step_args(sc: &serde_json::Value, i: usize) -> Vec<String> {
    sc["steps"][i]["args"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a.as_str().unwrap().to_string())
        .collect()
}

pub fn step_exit(sc: &serde_json::Value, i: usize) -> i64 {
    sc["steps"][i]["exit"].as_i64().unwrap()
}

pub fn scenario_len(sc: &serde_json::Value) -> usize {
    sc["steps"].as_array().unwrap().len()
}

pub fn node(kind: Kind, id: &str) -> Node {
    Node::new(kind, id.to_string())
}

pub fn work(id: &str, wtype: &str, status: &str, cynefin: &str) -> Node {
    let mut n = node(Kind::W, id);
    n.wtype = Some(wtype.to_string());
    n.status = status.to_string();
    n.cynefin = Some(cynefin.to_string());
    n
}

pub fn plain(kind: Kind, id: &str, status: &str) -> Node {
    let mut n = node(kind, id);
    n.status = status.to_string();
    n
}

pub fn prose(n: &mut Node, key: &str, lines: &[&str]) {
    n.fields.insert(
        key.to_string(),
        FieldValue::Prose(lines.iter().map(|s| s.to_string()).collect()),
    );
}

pub fn reflist(n: &mut Node, key: &str, items: &[&str]) {
    n.fields.insert(
        key.to_string(),
        FieldValue::RefList(items.iter().map(|s| s.to_string()).collect()),
    );
}

pub fn single(n: &mut Node, key: &str, v: &str) {
    n.fields.insert(
        key.to_string(),
        FieldValue::Single(v.to_string()),
    );
}

pub fn fitness(n: &mut Node, pairs: &[(&str, i64)]) {
    let m: BTreeMap<String, i64> = pairs
        .iter()
        .map(|(k, v)| (k.to_string(), *v))
        .collect();
    n.fields.insert("fitness".to_string(), FieldValue::Fitness(m));
}

pub fn attr(n: &mut Node, key: &str, v: &str) {
    n.attrs.insert(key.to_string(), v.to_string());
}

pub fn edge(st: &mut State, from: &str, label: &str, to: &str) {
    st.edges.push(Edge::new(from, label, to));
}

pub fn put(st: &mut State, n: Node) {
    st.nodes.insert(n.id.clone(), n);
}

pub fn dor_ready_feature(id: &str, status: &str) -> Node {
    let mut w = work(id, "feature", status, "clear");
    reflist(&mut w, "goals", &["G-01"]);
    reflist(&mut w, "ac", &["a"]);
    reflist(&mut w, "hypothesis", &["h"]);
    reflist(&mut w, "evidence_strategy", &["e"]);
    fitness(&mut w, &[("G-01", 1)]);
    w
}
