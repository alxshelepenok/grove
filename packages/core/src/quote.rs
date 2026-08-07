pub(crate) fn needs_quote(s: &str) -> bool {
    s.is_empty()
        || s
            .chars()
            .any(|c| matches!(c, ' ' | '"' | '\\' | '\t' | '\n'))
}

pub(crate) fn quote_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

pub(crate) fn maybe_quote(s: &str) -> String {
    if needs_quote(s) {
        quote_str(s)
    } else {
        s.to_string()
    }
}
