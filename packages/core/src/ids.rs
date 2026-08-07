use crate::model::{Kind, State};

pub fn family_prefix(kind: Kind) -> char {
    match kind {
        Kind::G => 'G',
        Kind::W => 'W',
        Kind::D => 'D',
        Kind::Q => 'Q',
        Kind::B => 'B',
        Kind::T => 'T',
        Kind::Y => 'Y',
        Kind::A => 'A',
    }
}

pub fn parse_id_numeric(id: &str) -> Result<(char, i64), String> {
    let s = id.trim();
    let b = s.as_bytes();
    let valid = b.len() >= 3
        && b[0].is_ascii_uppercase()
        && b[1] == b'-'
        && b[2..].iter().all(|c| c.is_ascii_digit())
        && b[2..].iter().any(|c| *c != b'0');
    if !valid {
        return Err(format!("malformed id: {id}"));
    }
    let n: i64 = s[2..].parse().map_err(|_| format!("malformed id: {id}"))?;
    Ok((b[0] as char, n))
}

pub fn format_allocated_id(prefix: char, numeric: i64, min_pad: i64) -> String {
    let digits = if numeric <= 0 {
        1
    } else {
        numeric.to_string().len() as i64
    };
    let w = 2.max(min_pad).max(digits) as usize;
    format!("{prefix}-{numeric:0>w$}")
}

pub fn next_id(st: &mut State, kind: Kind) -> String {
    let stride = st.id_stride.max(1);
    let off = st.id_offset.max(1);
    let prefix = family_prefix(kind);
    let cur = st.counters.get(&prefix).copied().unwrap_or(0);
    let nextnum = if cur <= 0 { off } else { cur + stride };
    st.counters.insert(prefix, nextnum);
    format_allocated_id(prefix, nextnum, st.id_pad_width)
}

pub fn record_id(st: &mut State, id: &str) {
    if id.is_empty() {
        return;
    }
    if let Ok((p, n)) = parse_id_numeric(id) {
        let cur = st.counters.get(&p).copied().unwrap_or(0);
        if n > cur {
            st.counters.insert(p, n);
        }
    }
}

pub fn reconcile_counters(st: &mut State) {
    st.counters.clear();
    let mut ids: Vec<String> = st.nodes.keys().cloned().collect();
    for e in &st.edges {
        ids.push(e.from.clone());
        ids.push(e.to.clone());
    }
    for id in ids {
        record_id(st, &id);
    }
}
