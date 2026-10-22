#[derive(Clone, Debug, PartialEq)]
pub(crate) enum TomlVal {
    Str(String),
    Arr(Vec<TomlVal>),
    Table(Vec<(String, TomlVal)>),
    Other,
}

#[derive(Clone, Copy)]
pub(crate) struct LinePolicy {
    pub dotted_headers: bool,
    pub arrays_of_tables: bool,
}

pub(crate) const REGISTRY_LINE_POLICY: LinePolicy = LinePolicy {
    dotted_headers: false,
    arrays_of_tables: true,
};

pub(crate) const CONFIG_LINE_POLICY: LinePolicy = LinePolicy {
    dotted_headers: true,
    arrays_of_tables: false,
};

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Line {
    Blank,
    Header(String),
    ArrayHeader(String),
    Assign(String, TomlVal),
}

pub(crate) fn bare_key(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

fn header_key(key: &str, policy: LinePolicy) -> bool {
    if policy.dotted_headers {
        !key.is_empty() && key.split('.').all(bare_key)
    } else {
        bare_key(key)
    }
}

fn parse_basic_string(s: &str) -> Result<(String, &str), ()> {
    let b = s.as_bytes();
    if b.first() != Some(&b'"') {
        return Err(());
    }
    let mut out = String::new();
    let mut i = 1;
    while i < b.len() {
        match b[i] {
            b'"' => return Ok((out, &s[i + 1..])),
            b'\\' => {
                i += 1;
                if i >= b.len() {
                    return Err(());
                }
                match b[i] {
                    b'"' => out.push('"'),
                    b'\\' => out.push('\\'),
                    b'n' => out.push('\n'),
                    b't' => out.push('\t'),
                    b'r' => out.push('\r'),
                    b'b' => out.push('\u{8}'),
                    b'f' => out.push('\u{c}'),
                    b'u' | b'U' => {
                        let n = if b[i] == b'u' { 4 } else { 8 };
                        let lo = i + 1;
                        let hi = lo + n;
                        if hi > b.len() {
                            return Err(());
                        }
                        let hex = &s[lo..hi];
                        if !hex.bytes().all(|c| c.is_ascii_hexdigit()) {
                            return Err(());
                        }
                        let cp = u32::from_str_radix(hex, 16).map_err(|_| ())?;
                        out.push(char::from_u32(cp).ok_or(())?);
                        i = hi - 1;
                    }
                    _ => return Err(()),
                }
                i += 1;
            }
            0x00..=0x08 | 0x0a..=0x1f | 0x7f => return Err(()),
            _ => {
                let ch = s[i..].chars().next().ok_or(())?;
                out.push(ch);
                i += ch.len_utf8();
            }
        }
    }
    Err(())
}

fn scalar_value(cand: &str) -> Result<TomlVal, ()> {
    if cand.is_empty()
        || !cand
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "+-_.:".contains(c))
    {
        return Err(());
    }
    let ok = cand.chars().any(|c| c.is_ascii_digit())
        || matches!(
            cand,
            "true" | "false" | "inf" | "nan" | "+inf" | "-inf" | "+nan" | "-nan"
        );
    if !ok {
        return Err(());
    }
    Ok(TomlVal::Other)
}

pub(crate) fn parse_value(s: &str) -> Result<(TomlVal, &str), ()> {
    let t = s.trim_start();
    if t.starts_with('"') {
        let (v, rest) = parse_basic_string(t)?;
        return Ok((TomlVal::Str(v), rest));
    }
    if t.starts_with('[') {
        return parse_inline_array(t);
    }
    if t.starts_with('{') {
        return parse_inline_table(t);
    }
    let end = t.find([',', ']', '}', '#']).unwrap_or(t.len());
    let v = scalar_value(t[..end].trim())?;
    Ok((v, &t[end..]))
}

fn parse_inline_array(s: &str) -> Result<(TomlVal, &str), ()> {
    let mut rest = s[1..].trim_start();
    let mut items = Vec::new();
    if let Some(r) = rest.strip_prefix(']') {
        return Ok((TomlVal::Arr(items), r));
    }
    loop {
        let (v, r) = parse_value(rest)?;
        items.push(v);
        rest = r.trim_start();
        if let Some(r2) = rest.strip_prefix(',') {
            rest = r2.trim_start();
            if let Some(r3) = rest.strip_prefix(']') {
                return Ok((TomlVal::Arr(items), r3));
            }
            continue;
        }
        if let Some(r2) = rest.strip_prefix(']') {
            return Ok((TomlVal::Arr(items), r2));
        }
        return Err(());
    }
}

fn parse_inline_table(s: &str) -> Result<(TomlVal, &str), ()> {
    let mut rest = s[1..].trim_start();
    let mut pairs: Vec<(String, TomlVal)> = Vec::new();
    if let Some(r) = rest.strip_prefix('}') {
        return Ok((TomlVal::Table(pairs), r));
    }
    loop {
        let eq = rest.find('=').ok_or(())?;
        let key = rest[..eq].trim();
        if !bare_key(key) {
            return Err(());
        }
        let (v, r) = parse_value(&rest[eq + 1..])?;
        if pairs.iter().any(|(k, _)| k == key) {
            return Err(());
        }
        pairs.push((key.to_string(), v));
        rest = r.trim_start();
        if let Some(r2) = rest.strip_prefix(',') {
            rest = r2.trim_start();
            continue;
        }
        if let Some(r2) = rest.strip_prefix('}') {
            return Ok((TomlVal::Table(pairs), r2));
        }
        return Err(());
    }
}

pub(crate) fn parse_line(line: &str, policy: LinePolicy) -> Result<Line, ()> {
    let t = line.trim();
    if t.is_empty() || t.starts_with('#') {
        return Ok(Line::Blank);
    }
    if let Some(rest) = t.strip_prefix("[[") {
        if !policy.arrays_of_tables {
            return Err(());
        }
        let close = rest.find("]]").ok_or(())?;
        let key = rest[..close].trim();
        let after = rest[close + 2..].trim();
        if (!after.is_empty() && !after.starts_with('#')) || !header_key(key, policy) {
            return Err(());
        }
        return Ok(Line::ArrayHeader(key.to_string()));
    }
    if let Some(rest) = t.strip_prefix('[') {
        let close = rest.find(']').ok_or(())?;
        let key = rest[..close].trim();
        let after = rest[close + 1..].trim();
        if (!after.is_empty() && !after.starts_with('#')) || !header_key(key, policy) {
            return Err(());
        }
        return Ok(Line::Header(key.to_string()));
    }
    let eq = t.find('=').ok_or(())?;
    let key = t[..eq].trim();
    if !bare_key(key) {
        return Err(());
    }
    let (v, rest) = parse_value(&t[eq + 1..])?;
    let after = rest.trim();
    if !after.is_empty() && !after.starts_with('#') {
        return Err(());
    }
    Ok(Line::Assign(key.to_string(), v))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_policy_rejects_dotted_headers() {
        assert!(parse_line("[roles.worker]", REGISTRY_LINE_POLICY).is_err());
        assert!(parse_line("[projects]", REGISTRY_LINE_POLICY).is_ok());
    }

    #[test]
    fn config_policy_accepts_dotted_headers() {
        assert_eq!(
            parse_line("[roles.worker]", CONFIG_LINE_POLICY).unwrap(),
            Line::Header("roles.worker".to_string())
        );
    }

    #[test]
    fn config_policy_rejects_arrays_of_tables() {
        assert!(parse_line("[[projects]]", CONFIG_LINE_POLICY).is_err());
    }

    #[test]
    fn registry_policy_accepts_arrays_of_tables() {
        assert_eq!(
            parse_line("[[projects]] # trailing", REGISTRY_LINE_POLICY).unwrap(),
            Line::ArrayHeader("projects".to_string())
        );
    }

    #[test]
    fn assign_lines_are_shared_across_policies() {
        assert_eq!(
            parse_line("disabled = [\"undo\", \"repair\"] # block", CONFIG_LINE_POLICY).unwrap(),
            Line::Assign(
                "disabled".to_string(),
                TomlVal::Arr(vec![
                    TomlVal::Str("undo".to_string()),
                    TomlVal::Str("repair".to_string())
                ])
            )
        );
        assert_eq!(
            parse_line("future = 5", REGISTRY_LINE_POLICY).unwrap(),
            Line::Assign("future".to_string(), TomlVal::Other)
        );
        assert!(parse_line("bad = [\"x\"", CONFIG_LINE_POLICY).is_err());
        assert!(parse_line("bad key = 1", REGISTRY_LINE_POLICY).is_err());
    }
}
