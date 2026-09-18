use crate::model::{Kind, State};
use std::cmp::Ordering;

pub fn id_sort_key(id: &str) -> (char, i64, &str) {
    let fallback = |num: i64| (id.chars().next().unwrap_or('\0'), num, id);
    let mut parts = id.splitn(2, '-');
    let (family, rest) = match (parts.next(), parts.next()) {
        (Some(f), Some(r)) => (f, r),
        _ => return fallback(i64::MAX),
    };
    let mut family_chars = family.chars();
    let family = match (family_chars.next(), family_chars.next()) {
        (Some(c), None) => c,
        _ => return fallback(i64::MAX),
    };
    if !family.is_ascii_uppercase() || rest.is_empty() || !rest.bytes().all(|b| b.is_ascii_digit())
    {
        return fallback(i64::MAX);
    }
    match rest.parse::<i64>() {
        Ok(n) => (family, n, id),
        Err(_) => fallback(i64::MAX),
    }
}

pub fn id_cmp(a: &str, b: &str) -> Ordering {
    id_sort_key(a).cmp(&id_sort_key(b))
}

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

#[cfg(test)]
mod tests {
    use super::id_cmp;

    fn ordered(ids: &[&str]) -> bool {
        ids.windows(2).all(|w| id_cmp(w[0], w[1]) != std::cmp::Ordering::Greater)
    }

    #[test]
    fn sorts_numbers_not_strings() {
        assert!(ordered(&["W-9", "W-10", "W-99", "W-100", "W-101"]));
        assert!(ordered(&["G-9", "G-10"]));
        assert!(ordered(&["A-2", "A-10"]));
    }

    #[test]
    fn families_group_by_prefix() {
        assert!(ordered(&["A-10", "B-2", "D-99", "G-3", "Q-100", "T-4", "W-100", "Y-5"]));
    }

    #[test]
    fn zero_and_padding_fall_back_to_string() {
        assert!(ordered(&["W-0", "W-1"]));
        assert!(ordered(&["W-0", "W-00"]));
    }

    #[test]
    fn malformed_ids_sort_after_numeric_same_family() {
        assert!(ordered(&["W-100", "W-bogus"]));
        assert!(ordered(&["W-!", "W-bogus"]));
        assert_eq!(id_cmp("W-bogus", "W-bogus"), std::cmp::Ordering::Equal);
    }

    #[test]
    fn empty_and_lowercase_families_are_total() {
        assert!(ordered(&["", "A-1"]));
        assert!(ordered(&["Z-1", "a-1"]));
        assert!(ordered(&["AB-1", "AB-2"]));
    }
}
