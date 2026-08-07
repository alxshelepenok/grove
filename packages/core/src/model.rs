use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    G,
    W,
    D,
    Q,
    B,
    T,
    Y,
    A,
}

impl Kind {
    pub const ALL: [Kind; 8] = [
        Kind::G,
        Kind::W,
        Kind::D,
        Kind::Q,
        Kind::B,
        Kind::T,
        Kind::Y,
        Kind::A,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Kind::G => "g",
            Kind::W => "w",
            Kind::D => "d",
            Kind::Q => "q",
            Kind::B => "b",
            Kind::T => "t",
            Kind::Y => "y",
            Kind::A => "a",
        }
    }
}

impl std::str::FromStr for Kind {
    type Err = ();

    fn from_str(s: &str) -> Result<Kind, ()> {
        Ok(match s {
            "g" => Kind::G,
            "w" => Kind::W,
            "d" => Kind::D,
            "q" => Kind::Q,
            "b" => Kind::B,
            "t" => Kind::T,
            "y" => Kind::Y,
            "a" => Kind::A,
            _ => return Err(()),
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Form {
    Prose,
    RefList,
    Single,
    Fitness,
}

pub fn field_form(kind: Kind, key: &str) -> Option<Form> {
    use Form::{Fitness, Prose, RefList, Single};
    use Kind::{A, B, D, G, Q, T, W, Y};
    Some(match (kind, key) {
        (W | D | Q | B | G | T | Y, "tags") => RefList,
        (W, "goals") => RefList,
        (W, "theme") => Single,
        (W, "fitness") => Fitness,
        (W, "surface") => RefList,
        (W, "ac" | "hypothesis" | "repro" | "exit" | "evidence_strategy" | "evidence" | "plan"
        | "why") => Prose,
        (D, "context" | "options" | "decision" | "consequences" | "validation") => Prose,
        (Q, "why" | "hypothesis" | "exit" | "log" | "outcome") => Prose,
        (B, "vm" | "threshold" | "result") => Prose,
        (G, "area") => Single,
        (G, "notes") => Prose,
        (G, "fitness_target" | "fitness_current") => Single,
        (T, "notes") => Prose,
        (Y, "surface") => RefList,
        (Y, "invariant" | "why" | "skill_updates" | "glossary_updates" | "revalidation") => Prose,
        (A, "surface") => RefList,
        _ => return None,
    })
}

pub fn field_order(kind: Kind) -> &'static [&'static str] {
    match kind {
        Kind::W => &[
            "goals",
            "theme",
            "fitness",
            "tags",
            "surface",
            "ac",
            "hypothesis",
            "repro",
            "exit",
            "evidence_strategy",
            "evidence",
            "plan",
            "why",
        ],
        Kind::D => &["tags", "context", "options", "decision", "consequences", "validation"],
        Kind::Q => &["tags", "why", "hypothesis", "exit", "log", "outcome"],
        Kind::B => &["tags", "vm", "threshold", "result"],
        Kind::G => &["area", "tags", "fitness_target", "fitness_current", "notes"],
        Kind::T => &["tags", "notes"],
        Kind::Y => &[
            "tags",
            "surface",
            "invariant",
            "why",
            "skill_updates",
            "glossary_updates",
            "revalidation",
        ],
        Kind::A => &["surface"],
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FieldValue {
    Prose(Vec<String>),
    RefList(Vec<String>),
    Single(String),
    Fitness(BTreeMap<String, i64>),
}

#[derive(Clone, Debug)]
pub struct Node {
    pub kind: Kind,
    pub id: String,
    pub title: String,
    pub wtype: Option<String>,
    pub status: String,
    pub cynefin: Option<String>,
    pub attrs: BTreeMap<String, String>,
    pub fields: BTreeMap<String, FieldValue>,
    pub archived: bool,
}

impl Node {
    pub fn new(kind: Kind, id: String) -> Node {
        Node {
            kind,
            id,
            title: String::new(),
            wtype: None,
            status: "proposed".to_string(),
            cynefin: None,
            attrs: BTreeMap::new(),
            fields: BTreeMap::new(),
            archived: false,
        }
    }

    pub fn lines(&self, key: &str) -> Vec<String> {
        match self.fields.get(key) {
            Some(FieldValue::Prose(v)) | Some(FieldValue::RefList(v)) => v.clone(),
            Some(FieldValue::Single(s)) if !s.is_empty() => vec![s.clone()],
            _ => Vec::new(),
        }
    }

    pub fn single(&self, key: &str) -> String {
        match self.fields.get(key) {
            Some(FieldValue::Single(s)) => s.clone(),
            _ => String::new(),
        }
    }

    pub fn set_single(&mut self, key: &str, value: String) {
        self.fields.insert(key.to_string(), FieldValue::Single(value));
    }

    pub fn fitness(&self) -> BTreeMap<String, i64> {
        match self.fields.get("fitness") {
            Some(FieldValue::Fitness(m)) => m.clone(),
            _ => BTreeMap::new(),
        }
    }

    pub fn attr(&self, key: &str) -> String {
        self.attrs.get(key).cloned().unwrap_or_default()
    }
}

#[derive(Clone, Debug)]
pub struct Edge {
    pub from: String,
    pub label: String,
    pub to: String,
    pub t_created: Option<String>,
}

impl Edge {
    pub fn new(from: &str, label: &str, to: &str) -> Edge {
        Edge {
            from: from.to_string(),
            label: label.to_string(),
            to: to.to_string(),
            t_created: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct State {
    pub nodes: BTreeMap<String, Node>,
    pub edges: Vec<Edge>,
    pub counters: BTreeMap<char, i64>,
    pub id_stride: i64,
    pub id_offset: i64,
    pub id_pad_width: i64,
}

impl Default for State {
    fn default() -> State {
        State {
            nodes: BTreeMap::new(),
            edges: Vec::new(),
            counters: BTreeMap::new(),
            id_stride: 1,
            id_offset: 1,
            id_pad_width: 2,
        }
    }
}
