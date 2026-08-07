use crate::model::{field_order, Edge, FieldValue, Kind, Node, State};
use crate::quote::{maybe_quote, quote_str};

pub fn serialize_body(st: &State) -> String {
    let mut out = String::new();
    if !(st.id_stride == 1 && st.id_offset == 1 && st.id_pad_width == 2) {
        out.push_str(&format!(
            "# @grove-id stride={} offset={} pad={}\n\n",
            st.id_stride, st.id_offset, st.id_pad_width
        ));
    }
    for archived in [false, true] {
        let mut first_in_section = true;
        if archived {
            if !st.nodes.values().any(|n| n.archived) {
                break;
            }
            if out.is_empty() {
                out.push_str(":archive\n");
            } else {
                out.push_str("\n:archive\n");
            }
        }
        for kind in Kind::ALL {
            let mut nodes: Vec<&Node> = st.nodes.values().filter(|n| n.kind == kind).collect();
            nodes.sort_by(|a, b| a.id.cmp(&b.id));
            for n in nodes {
                if n.archived != archived {
                    continue;
                }
                if !first_in_section {
                    out.push('\n');
                }
                first_in_section = false;
                serialize_node(&mut out, n);
            }
        }
        if !archived {
            let mut edges: Vec<&Edge> = st.edges.iter().collect();
            edges.sort_by(|a, b| (&a.from, &a.label, &a.to).cmp(&(&b.from, &b.label, &b.to)));
            if !edges.is_empty() {
                out.push('\n');
                for e in edges {
                    out.push_str("e ");
                    out.push_str(&e.from);
                    out.push(' ');
                    out.push_str(&e.label);
                    out.push(' ');
                    out.push_str(&e.to);
                    out.push_str(" t_created=");
                    out.push_str(&maybe_quote(e.t_created.as_deref().unwrap_or("")));
                    out.push('\n');
                }
            }
        }
    }
    out
}

pub fn serialize(st: &State) -> String {
    let body = serialize_body(st);
    let cks = crate::checksum_of(&body);
    let mut out = String::with_capacity(body.len() + 128);
    out.push_str("@grove 1\n# AUTO-GENERATED. Do not edit. Use `grove` CLI.\n# checksum: sha256:");
    out.push_str(&cks);
    out.push('\n');
    out.push_str(&body);
    if !body.ends_with('\n') {
        out.push('\n');
    }
    out
}

pub fn serialize_node_string(n: &Node) -> String {
    let mut out = String::new();
    serialize_node(&mut out, n);
    out
}

fn serialize_node(out: &mut String, n: &Node) {
    out.push_str(n.kind.as_str());
    out.push(' ');
    out.push_str(&n.id);
    match n.kind {
        Kind::W => {
            if let Some(t) = &n.wtype {
                out.push_str(" type=");
                out.push_str(t);
            }
            out.push_str(" status=");
            out.push_str(&n.status);
            if let Some(c) = &n.cynefin {
                out.push_str(" cynefin=");
                out.push_str(c);
            }
        }
        Kind::G => {
            out.push_str(" status=");
            out.push_str(&n.status);
            if let Some(f) = n.attrs.get("fitness") {
                out.push_str(" fitness=");
                out.push_str(&maybe_quote(f));
            }
        }
        Kind::D | Kind::T | Kind::Y | Kind::A => {
            out.push_str(" status=");
            out.push_str(&n.status);
        }
        Kind::Q | Kind::B => {
            out.push_str(" status=");
            out.push_str(&n.status);
            if let Some(c) = &n.cynefin {
                out.push_str(" cynefin=");
                out.push_str(c);
            }
        }
    }
    for (k, v) in &n.attrs {
        if matches!(k.as_str(), "fitness" | "goal" | "date") {
            continue;
        }
        out.push(' ');
        out.push_str(k);
        out.push('=');
        out.push_str(&maybe_quote(v));
    }
    if !n.title.is_empty() {
        out.push(' ');
        out.push_str(&quote_str(&n.title));
    }
    out.push('\n');
    for fname in field_order(n.kind) {
        let Some(v) = n.fields.get(*fname) else {
            continue;
        };
        match v {
            FieldValue::Prose(lines) => {
                if lines.is_empty() {
                    continue;
                }
                out.push_str("  ");
                out.push_str(fname);
                out.push_str(":\n");
                for line in lines {
                    out.push_str("    | ");
                    out.push_str(line);
                    out.push('\n');
                }
            }
            FieldValue::RefList(items) => {
                if items.is_empty() {
                    continue;
                }
                out.push_str("  ");
                out.push_str(fname);
                out.push_str(": ");
                out.push_str(&items.join(", "));
                out.push('\n');
            }
            FieldValue::Single(s) => {
                if s.is_empty() {
                    continue;
                }
                out.push_str("  ");
                out.push_str(fname);
                out.push_str(": ");
                out.push_str(s);
                out.push('\n');
            }
            FieldValue::Fitness(deltas) => {
                if deltas.is_empty() {
                    continue;
                }
                let parts: Vec<String> = deltas
                    .iter()
                    .map(|(k, d)| format!("{}={}{}", k, if *d >= 0 { "+" } else { "" }, d))
                    .collect();
                out.push_str("  ");
                out.push_str(fname);
                out.push_str(": ");
                out.push_str(&parts.join(", "));
                out.push('\n');
            }
        }
    }
}
