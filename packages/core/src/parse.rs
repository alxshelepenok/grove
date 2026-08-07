use crate::model::{field_form, Edge, FieldValue, Form, Kind, Node, State};
use crate::quote::quote_str;
use std::collections::BTreeMap;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseMode {
    Strict,
    Fixture,
}

#[derive(Clone, Debug)]
pub struct ParseError {
    pub msg: String,
    pub line: usize,
}

impl ParseError {
    fn new(msg: impl Into<String>, line: usize) -> ParseError {
        ParseError {
            msg: msg.into(),
            line,
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "lock parse error at line {}: {}", self.line, self.msg)
    }
}

impl std::error::Error for ParseError {}

const MAGIC: &str = "@grove 1";
const CHECKSUM_PREFIX: &str = "# checksum: sha256:";

pub fn parse(text: &str, mode: ParseMode) -> Result<State, ParseError> {
    parse_internal(text, mode).map(|(st, _, _)| st)
}

pub fn parse_strict(text: &str) -> Result<State, ParseError> {
    parse(text, ParseMode::Strict)
}

pub fn parse_fixture(text: &str) -> Result<State, ParseError> {
    parse(text, ParseMode::Fixture)
}

pub(crate) fn parse_internal(
    text: &str,
    mode: ParseMode,
) -> Result<(State, String, String), ParseError> {
    let text = text.replace("\r\n", "\n");
    let lines: Vec<&str> = text.split('\n').collect();
    let mut st = State::default();

    if lines.len() < 3 {
        return Err(ParseError::new("file too short", 0));
    }
    if lines[0].trim() != MAGIC {
        return Err(ParseError::new(format!("missing magic '{MAGIC}'"), 1));
    }
    if !lines[1].starts_with('#') {
        return Err(ParseError::new("missing header comment", 2));
    }
    if !lines[2].starts_with(CHECKSUM_PREFIX) {
        return Err(ParseError::new("missing checksum", 3));
    }
    let expected = lines[2][CHECKSUM_PREFIX.len()..].trim().to_string();
    if mode == ParseMode::Strict && !is_hex64(&expected) {
        return Err(ParseError::new("checksum is not 64 lowercase hex", 3));
    }

    let mut body_start = 3;
    while body_start < lines.len() && lines[body_start].is_empty() {
        body_start += 1;
    }
    let body = lines[body_start..].join("\n");

    let mut scan_from = body_start;
    while scan_from < lines.len() {
        let raw = lines[scan_from];
        if raw.trim().is_empty() {
            scan_from += 1;
            continue;
        }
        match parse_id_meta(raw) {
            Some((stride, offset, pad)) => {
                st.id_stride = stride;
                st.id_offset = offset;
                st.id_pad_width = pad;
                scan_from += 1;
            }
            None => break,
        }
    }

    let mut in_archive = false;
    let mut cur_node: Option<String> = None;
    let mut cur_field: Option<String> = None;
    let mut i = scan_from;
    while i < lines.len() {
        let raw = lines[i];
        let ln = i + 1;
        i += 1;
        if raw.is_empty() {
            cur_field = None;
            continue;
        }
        if raw.starts_with("# ") {
            cur_field = None;
            continue;
        }
        if raw == ":archive" {
            in_archive = true;
            cur_node = None;
            cur_field = None;
            continue;
        }
        if raw.starts_with("    | ") || raw == "    |" {
            let id = cur_node
                .clone()
                .ok_or_else(|| ParseError::new("prose without record", ln))?;
            let key = cur_field
                .clone()
                .ok_or_else(|| ParseError::new("prose without field", ln))?;
            let prose = if raw.len() > 6 { &raw[6..] } else { "" };
            let node = st.nodes.get_mut(&id).expect("current node exists");
            match node.fields.get_mut(&key) {
                Some(FieldValue::Prose(v)) | Some(FieldValue::RefList(v)) => {
                    v.push(prose.to_string())
                }
                _ => {
                    return Err(ParseError::new(
                        format!("field '{key}' cannot hold prose lines"),
                        ln,
                    ))
                }
            }
            continue;
        }
        if raw.starts_with("  ")
            && !raw.starts_with("   ")
            && raw.len() > 2
            && raw.as_bytes()[2] != b' '
        {
            let id = cur_node
                .clone()
                .ok_or_else(|| ParseError::new("field without record", ln))?;
            let colon = raw
                .find(':')
                .ok_or_else(|| ParseError::new("missing ':' in field", ln))?;
            let key = raw[2..colon].trim().to_string();
            let value = raw[colon + 1..].trim();
            let kind = st.nodes[&id].kind;
            let form = field_form(kind, &key).ok_or_else(|| {
                ParseError::new(format!("unknown field '{key}' on {}", kind.as_str()), ln)
            })?;
            cur_field = Some(key.clone());
            let node = st.nodes.get_mut(&id).expect("current node exists");
            match form {
                Form::Prose => {
                    node.fields.insert(key, FieldValue::Prose(Vec::new()));
                    if !value.is_empty() {
                        return Err(ParseError::new(
                            "prose field must not have inline value",
                            ln,
                        ));
                    }
                }
                Form::RefList => {
                    let items = if value.is_empty() {
                        Vec::new()
                    } else {
                        value.split(',').map(|s| s.trim().to_string()).collect()
                    };
                    node.fields.insert(key, FieldValue::RefList(items));
                }
                Form::Single => {
                    node.fields.insert(key, FieldValue::Single(value.to_string()));
                }
                Form::Fitness => {
                    let mut deltas = BTreeMap::new();
                    if !value.is_empty() {
                        for part in value.split(',') {
                            let part = part.trim();
                            let eq = part.find('=').ok_or_else(|| {
                                ParseError::new(format!("bad fitness entry '{part}'"), ln)
                            })?;
                            let gid = part[..eq].trim().to_string();
                            let raw_delta = part[eq + 1..].trim();
                            let delta: i64 = raw_delta.parse().map_err(|_| {
                                ParseError::new(format!("bad fitness delta '{raw_delta}'"), ln)
                            })?;
                            deltas.insert(gid, delta);
                        }
                    }
                    node.fields.insert(key, FieldValue::Fitness(deltas));
                }
            }
            continue;
        }
        cur_field = None;
        let toks = tokenize_header(raw).map_err(|msg| ParseError::new(msg, ln))?;
        if toks.is_empty() {
            continue;
        }
        let kind_str = match &toks[0] {
            Token::Bare(s) => s.clone(),
            _ => return Err(ParseError::new("expected record kind", ln)),
        };
        if kind_str == "e" {
            if toks.len() < 4 {
                return Err(ParseError::new("malformed edge", ln));
            }
            let from = toks[1].payload().to_string();
            let label = toks[2].payload().to_string();
            let to = toks[3].payload().to_string();
            let mut t_created = None;
            for t in &toks[4..] {
                match t {
                    Token::Eq(k, v, quoted) => {
                        if k == "t_created" {
                            if t_created.is_some() {
                                return Err(ParseError::new("duplicate t_created on edge", ln));
                            }
                            strict_attr(mode, k, v, *quoted, ln)?;
                            t_created = Some(v.clone());
                        } else {
                            return Err(ParseError::new(
                                format!("unknown edge attribute '{k}' (only t_created allowed)"),
                                ln,
                            ));
                        }
                    }
                    _ => return Err(ParseError::new("unexpected edge token", ln)),
                }
            }
            check_id(&from, ln)?;
            check_id(&to, ln)?;
            st.edges.push(Edge {
                from,
                label,
                to,
                t_created,
            });
            cur_node = None;
            continue;
        }
        let kind = kind_str
            .parse::<Kind>()
            .ok()
            .ok_or_else(|| ParseError::new(format!("unknown record kind '{kind_str}'"), ln))?;
        if in_archive && kind == Kind::Y {
            return Err(ParseError::new("y record in :archive section", ln));
        }
        if toks.len() < 2 {
            return Err(ParseError::new("missing id", ln));
        }
        let id = match &toks[1] {
            Token::Bare(s) => s.clone(),
            _ => return Err(ParseError::new("malformed id", ln)),
        };
        check_id(&id, ln)?;
        let mut node = Node::new(kind, id.clone());
        node.archived = in_archive;
        for t in &toks[2..] {
            match t {
                Token::Eq(k, v, quoted) => {
                    strict_attr(mode, k, v, *quoted, ln)?;
                    match k.as_str() {
                        "type" => node.wtype = Some(v.clone()),
                        "status" => node.status = v.clone(),
                        "cynefin" => node.cynefin = Some(v.clone()),
                        _ => {
                            node.attrs.insert(k.clone(), v.clone());
                        }
                    }
                }
                Token::Str(s) => node.title = s.clone(),
                Token::Bare(b) => {
                    return Err(ParseError::new(format!("unexpected token '{b}'"), ln))
                }
            }
        }
        if kind == Kind::T && node.status == "proposed" {
            node.status = "open".to_string();
        }
        if kind == Kind::A && node.status != "present" {
            return Err(ParseError::new(
                format!("a record status must be present (got '{}')", node.status),
                ln,
            ));
        }
        st.nodes.insert(id.clone(), node);
        cur_node = Some(id);
    }
    crate::ids::reconcile_counters(&mut st);
    Ok((st, body, expected))
}

fn is_hex64(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

fn is_iso8601_utc(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 20
        && b[4] == b'-'
        && b[7] == b'-'
        && b[10] == b'T'
        && b[13] == b':'
        && b[16] == b':'
        && b[19] == b'Z'
        && b
            .iter()
            .enumerate()
            .all(|(i, c)| matches!(i, 4 | 7 | 10 | 13 | 16 | 19) || c.is_ascii_digit())
}

fn strict_attr(
    mode: ParseMode,
    key: &str,
    value: &str,
    quoted: bool,
    line: usize,
) -> Result<(), ParseError> {
    if mode != ParseMode::Strict {
        return Ok(());
    }
    if !quoted
        && (value.is_empty()
            || value
                .chars()
                .any(|c| matches!(c, ' ' | '\t' | '\n' | '"' | '\\' | '<' | '>')))
    {
        return Err(ParseError::new(
            format!("invalid unquoted attr value '{value}' for '{key}'"),
            line,
        ));
    }
    if matches!(key, "t_created" | "t_updated" | "session_at") && !is_iso8601_utc(value) {
        return Err(ParseError::new(
            format!("invalid iso8601 utc timestamp '{value}' for '{key}'"),
            line,
        ));
    }
    Ok(())
}

fn check_id(id: &str, line: usize) -> Result<(), ParseError> {
    let s = id.trim();
    let b = s.as_bytes();
    let numeric_ok = b.len() > 2
        && b[2..].iter().all(|c| c.is_ascii_digit())
        && b[2..].iter().any(|c| *c != b'0');
    if b.len() >= 3 && b[0].is_ascii_uppercase() && b[1] == b'-' && numeric_ok {
        Ok(())
    } else {
        Err(ParseError::new(format!("malformed id: {id}"), line))
    }
}

fn take_ws(s: &str) -> Option<&str> {
    let t = s.trim_start_matches(char::is_whitespace);
    (t.len() != s.len()).then_some(t)
}

fn take_digits(s: &str) -> Option<(i64, &str)> {
    let end = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
    if end == 0 {
        return None;
    }
    match s[..end].parse() {
        Ok(n) => Some((n, &s[end..])),
        Err(_) => None,
    }
}

fn parse_id_meta(raw: &str) -> Option<(i64, i64, i64)> {
    let s = raw.strip_prefix('#')?;
    let s = take_ws(s)?;
    let s = s.strip_prefix("@grove-id")?;
    let s = take_ws(s)?;
    let s = s.strip_prefix("stride=")?;
    let (stride, s) = take_digits(s)?;
    let s = take_ws(s)?;
    let s = s.strip_prefix("offset=")?;
    let (offset, s) = take_digits(s)?;
    let s = take_ws(s)?;
    let s = s.strip_prefix("pad=")?;
    let (pad, s) = take_digits(s)?;
    if s.chars().any(|c| !c.is_whitespace()) {
        return None;
    }
    Some((stride, offset, pad))
}

enum Token {
    Bare(String),
    Str(String),
    Eq(String, String, bool),
}

impl Token {
    fn payload(&self) -> &str {
        match self {
            Token::Bare(s) | Token::Str(s) => s,
            Token::Eq(k, _, _) => k,
        }
    }
}

fn parse_qstring(s: &str, start: usize) -> Result<(String, usize), String> {
    let mut i = start + 1;
    let mut buf = String::new();
    while i < s.len() {
        let c = s[i..].chars().next().expect("index on boundary");
        if c == '"' {
            return Ok((buf, i + 1));
        }
        if c == '\\' && i + 1 < s.len() {
            let nc = s[i + 1..].chars().next().expect("index on boundary");
            match nc {
                '"' => buf.push('"'),
                '\\' => buf.push('\\'),
                'n' => buf.push('\n'),
                _ => return Err(format!("bad escape \\{nc}")),
            }
            i += 2;
        } else {
            buf.push(c);
            i += c.len_utf8();
        }
    }
    Err("unterminated quoted string".to_string())
}

fn tokenize_header(line: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let mut i = 0;
    let n = line.len();
    while i < n {
        let c = line[i..].chars().next().expect("index on boundary");
        if c == ' ' || c == '\t' {
            i += 1;
            continue;
        }
        if c == '"' {
            let (s, j) = parse_qstring(line, i)?;
            tokens.push(Token::Str(s));
            i = j;
            continue;
        }
        let mut buf = String::new();
        while i < n {
            let c = line[i..].chars().next().expect("index on boundary");
            if c == ' ' || c == '\t' {
                break;
            }
            if c == '"' {
                let (s, j) = parse_qstring(line, i)?;
                buf.push_str(&quote_str(&s));
                i = j;
            } else {
                buf.push(c);
                i += c.len_utf8();
            }
        }
        let tok = buf;
        match tok.find('=') {
            Some(eq) => {
                let key = &tok[..eq];
                let rest = &tok[eq + 1..];
                if rest.starts_with('"') {
                    let (s, _) = parse_qstring(rest, 0)?;
                    tokens.push(Token::Eq(key.to_string(), s, true));
                } else {
                    tokens.push(Token::Eq(key.to_string(), rest.to_string(), false));
                }
            }
            None => tokens.push(Token::Bare(tok)),
        }
    }
    Ok(tokens)
}
