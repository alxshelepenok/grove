use crate::jhash::julia_string_hash;

#[derive(Clone, Debug, PartialEq)]
pub enum JVal {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Arr(Vec<JVal>),
    Obj(JuliaDict),
}

#[derive(Clone, Debug, PartialEq)]
pub struct JuliaDict {
    slots: Vec<Option<(String, JVal)>>,
    count: usize,
}

fn tablesz(x: usize) -> usize {
    if x < 16 {
        16
    } else {
        1usize << (usize::BITS as usize - (x - 1).leading_zeros() as usize)
    }
}

impl JuliaDict {
    pub fn new() -> JuliaDict {
        JuliaDict {
            slots: Vec::new(),
            count: 0,
        }
    }

    pub fn from_pairs(pairs: Vec<(String, JVal)>) -> JuliaDict {
        let mut d = JuliaDict::new();
        d.sizehint(pairs.len());
        for (k, v) in pairs {
            d.insert(k, v);
        }
        d
    }

    pub fn with_sizehint(n: usize) -> JuliaDict {
        let mut d = JuliaDict::new();
        d.sizehint(n);
        d
    }

    pub fn slot_copy(other: &JuliaDict) -> JuliaDict {
        other.clone()
    }

    fn sizehint(&mut self, n: usize) {
        let newsz = tablesz((3 * n + 1) / 2);
        if newsz > self.slots.len() {
            self.rehash(newsz);
        }
    }

    fn rehash(&mut self, newsz: usize) {
        let newsz = tablesz(newsz);
        let old = std::mem::take(&mut self.slots);
        self.slots = vec![None; newsz];
        for slot in old.into_iter().flatten() {
            let idx = self.probe_empty(&slot.0);
            self.slots[idx] = Some(slot);
        }
    }

    fn hashidx(&self, key: &str) -> usize {
        (julia_string_hash(key) as usize) & (self.slots.len() - 1)
    }

    fn probe_empty(&self, key: &str) -> usize {
        let sz = self.slots.len();
        let mut idx = self.hashidx(key);
        while self.slots[idx].is_some() {
            idx = (idx + 1) & (sz - 1);
        }
        idx
    }

    pub fn insert(&mut self, key: String, val: JVal) {
        if self.slots.is_empty() {
            self.rehash(16);
        }
        let sz = self.slots.len();
        let maxallowed = 16usize.max(sz >> 6);
        let mut idx = self.hashidx(&key);
        let mut probed = 0usize;
        loop {
            match &self.slots[idx] {
                Some((k, _)) if *k == key => {
                    self.slots[idx] = Some((key, val));
                    return;
                }
                Some(_) => {
                    probed += 1;
                    if probed > maxallowed {
                        self.rehash(sz * 4);
                        self.insert(key, val);
                        return;
                    }
                    idx = (idx + 1) & (sz - 1);
                }
                None => break,
            }
        }
        self.slots[idx] = Some((key, val));
        self.count += 1;
        if self.count * 3 > sz * 2 {
            self.rehash(self.count * 4);
        }
    }

    pub fn merge_from(&mut self, other: &JuliaDict) {
        self.sizehint(self.count + other.count);
        let pairs: Vec<(String, JVal)> = other
            .iter_pairs()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        for (k, v) in pairs {
            self.insert(k, v);
        }
    }

    pub fn iter_pairs(&self) -> impl Iterator<Item = &(String, JVal)> {
        self.slots.iter().filter_map(|s| s.as_ref())
    }

    pub fn len(&self) -> usize {
        self.count
    }
}

pub fn julia_float_repr(x: f64) -> String {
    if x.is_nan() || x.is_infinite() {
        return "null".to_string();
    }
    if x == 0.0 {
        return if x.is_sign_negative() { "-0.0" } else { "0.0" }.to_string();
    }
    let sci = format!("{:e}", x);
    let (mant, exp_s) = sci.split_once('e').expect("lowerexp has e");
    let exp: i32 = exp_s.parse().expect("lowerexp exp");
    let neg = mant.starts_with('-');
    let mant = mant.trim_start_matches('-');
    let digits: String = mant.chars().filter(|c| *c != '.').collect();
    let dlen = digits.len() as i32;
    let point = exp + 1;
    let mut s = String::new();
    if neg {
        s.push('-');
    }
    if point > 6 || point < -3 {
        s.push(digits.as_bytes()[0] as char);
        s.push('.');
        if dlen == 1 {
            s.push('0');
        } else {
            s.push_str(&digits[1..]);
        }
        s.push('e');
        s.push_str(&(point - 1).to_string());
    } else if point <= 0 {
        s.push_str("0.");
        for _ in 0..(-point) {
            s.push('0');
        }
        s.push_str(&digits);
    } else if point >= dlen {
        s.push_str(&digits);
        for _ in 0..(point - dlen) {
            s.push('0');
        }
        s.push_str(".0");
    } else {
        s.push_str(&digits[..point as usize]);
        s.push('.');
        s.push_str(&digits[point as usize..]);
    }
    s
}

pub fn julia_round_digits2(x: f64) -> f64 {
    (x * 100.0).round_ties_even() / 100.0
}

pub fn julia_num_repr(x: f64) -> String {
    julia_float_repr(julia_round_digits2(x))
}

pub fn emit_jval(v: &JVal) -> String {
    let mut out = String::new();
    write_jval(&mut out, v);
    out
}

fn write_jval(out: &mut String, v: &JVal) {
    match v {
        JVal::Null => out.push_str("null"),
        JVal::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        JVal::Int(i) => out.push_str(&i.to_string()),
        JVal::Float(x) => out.push_str(&julia_float_repr(*x)),
        JVal::Str(s) => write_jstr(out, s),
        JVal::Arr(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_jval(out, item);
            }
            out.push(']');
        }
        JVal::Obj(d) => {
            out.push('{');
            for (i, (k, val)) in d.iter_pairs().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_jstr(out, k);
                out.push(':');
                write_jval(out, val);
            }
            out.push('}');
        }
    }
}

fn write_jstr(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{08}' => out.push_str("\\b"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\u{0c}' => out.push_str("\\f"),
            '\r' => out.push_str("\\r"),
            _ if (c as u32) < 0x20 || c == '\u{7f}' => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            _ => out.push(c),
        }
    }
    out.push('"');
}

#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(pairs) => pairs.iter().rev().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Json::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Json::Int(i) => Some(*i),
            Json::Float(f) => Some(f.round() as i64),
            _ => None,
        }
    }

    pub fn as_arr(&self) -> Option<&Vec<Json>> {
        match self {
            Json::Arr(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_obj(&self) -> Option<&Vec<(String, Json)>> {
        match self {
            Json::Obj(o) => Some(o),
            _ => None,
        }
    }
}

pub fn parse_json(text: &str) -> Result<Json, String> {
    let b = text.as_bytes();
    let mut p = Parser { b, i: 0 };
    p.skip_ws();
    let v = p.value()?;
    p.skip_ws();
    if p.i != b.len() {
        return Err(format!("trailing data at byte {}", p.i));
    }
    Ok(v)
}

struct Parser<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> Parser<'a> {
    fn skip_ws(&mut self) {
        while self.i < self.b.len() && matches!(self.b[self.i], b' ' | b'\t' | b'\n' | b'\r') {
            self.i += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.b.get(self.i).copied()
    }

    fn value(&mut self) -> Result<Json, String> {
        match self.peek() {
            None => Err("unexpected end of input".to_string()),
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b'"') => Ok(Json::Str(self.string()?)),
            Some(b't') => self.lit("true", Json::Bool(true)),
            Some(b'f') => self.lit("false", Json::Bool(false)),
            Some(b'n') => self.lit("null", Json::Null),
            Some(_) => self.number(),
        }
    }

    fn lit(&mut self, s: &str, v: Json) -> Result<Json, String> {
        if self.b.len() >= self.i + s.len() && &self.b[self.i..self.i + s.len()] == s.as_bytes() {
            self.i += s.len();
            Ok(v)
        } else {
            Err(format!("bad literal at byte {}", self.i))
        }
    }

    fn number(&mut self) -> Result<Json, String> {
        let start = self.i;
        if self.peek() == Some(b'-') {
            self.i += 1;
        }
        let mut float = false;
        while let Some(c) = self.peek() {
            match c {
                b'0'..=b'9' => self.i += 1,
                b'.' | b'e' | b'E' | b'+' | b'-' => {
                    float = true;
                    self.i += 1;
                }
                _ => break,
            }
        }
        let s = std::str::from_utf8(&self.b[start..self.i]).map_err(|_| "bad number")?;
        if s.is_empty() {
            return Err(format!("bad number at byte {start}"));
        }
        if !float {
            if let Ok(i) = s.parse::<i64>() {
                return Ok(Json::Int(i));
            }
        }
        s.parse::<f64>()
            .map(Json::Float)
            .map_err(|_| format!("bad number '{s}'"))
    }

    fn string(&mut self) -> Result<String, String> {
        if self.peek() != Some(b'"') {
            return Err(format!("expected string at byte {}", self.i));
        }
        self.i += 1;
        let mut out: Vec<u8> = Vec::new();
        loop {
            let c = self.peek().ok_or("unterminated string")?;
            self.i += 1;
            match c {
                b'"' => break,
                b'\\' => {
                    let e = self.peek().ok_or("unterminated escape")?;
                    self.i += 1;
                    match e {
                        b'"' => out.push(b'"'),
                        b'\\' => out.push(b'\\'),
                        b'/' => out.push(b'/'),
                        b'b' => out.push(0x08),
                        b'f' => out.push(0x0c),
                        b'n' => out.push(b'\n'),
                        b'r' => out.push(b'\r'),
                        b't' => out.push(b'\t'),
                        b'u' => {
                            let cp = self.hex4()?;
                            if (0xd800..0xdc00).contains(&cp) {
                                if self.peek() == Some(b'\\')
                                    && self.b.get(self.i + 1) == Some(&b'u')
                                {
                                    self.i += 2;
                                    let lo = self.hex4()?;
                                    let c = 0x10000 + ((cp - 0xd800) << 10) + (lo - 0xdc00);
                                    self.push_char(&mut out, c);
                                } else {
                                    self.push_char(&mut out, 0xfffd);
                                }
                            } else {
                                self.push_char(&mut out, cp);
                            }
                        }
                        _ => return Err(format!("bad escape \\{} at byte {}", e as char, self.i)),
                    }
                }
                _ => out.push(c),
            }
        }
        String::from_utf8(out).map_err(|_| "string is not valid utf-8".to_string())
    }

    fn push_char(&self, out: &mut Vec<u8>, cp: u32) {
        let c = char::from_u32(cp).unwrap_or('\u{fffd}');
        let mut buf = [0u8; 4];
        out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
    }

    fn hex4(&mut self) -> Result<u32, String> {
        if self.i + 4 > self.b.len() {
            return Err("bad \\u escape".to_string());
        }
        let s = std::str::from_utf8(&self.b[self.i..self.i + 4]).map_err(|_| "bad \\u escape")?;
        let v = u32::from_str_radix(s, 16).map_err(|_| "bad \\u escape")?;
        self.i += 4;
        Ok(v)
    }

    fn array(&mut self) -> Result<Json, String> {
        self.i += 1;
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.i += 1;
            return Ok(Json::Arr(items));
        }
        loop {
            self.skip_ws();
            items.push(self.value()?);
            self.skip_ws();
            match self.peek() {
                Some(b',') => self.i += 1,
                Some(b']') => {
                    self.i += 1;
                    return Ok(Json::Arr(items));
                }
                _ => return Err(format!("expected ',' or ']' at byte {}", self.i)),
            }
        }
    }

    fn object(&mut self) -> Result<Json, String> {
        self.i += 1;
        let mut pairs = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.i += 1;
            return Ok(Json::Obj(pairs));
        }
        loop {
            self.skip_ws();
            let k = self.string()?;
            self.skip_ws();
            if self.peek() != Some(b':') {
                return Err(format!("expected ':' at byte {}", self.i));
            }
            self.i += 1;
            self.skip_ws();
            let v = self.value()?;
            pairs.push((k, v));
            self.skip_ws();
            match self.peek() {
                Some(b',') => self.i += 1,
                Some(b'}') => {
                    self.i += 1;
                    return Ok(Json::Obj(pairs));
                }
                _ => return Err(format!("expected ',' or '}}' at byte {}", self.i)),
            }
        }
    }
}
