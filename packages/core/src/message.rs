use crate::json::{emit_jval, JVal, JuliaDict};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        }
    }

    pub fn parse(s: &str) -> Option<Severity> {
        match s {
            "error" => Some(Severity::Error),
            "warning" => Some(Severity::Warning),
            "info" => Some(Severity::Info),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Code {
    System,
    Interface,
    Invariant,
    Operational,
}

impl Code {
    pub const ALL: [Code; 4] = [
        Code::System,
        Code::Interface,
        Code::Invariant,
        Code::Operational,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Code::System => "SYSTEM",
            Code::Interface => "INTERFACE",
            Code::Invariant => "INVARIANT",
            Code::Operational => "OPERATIONAL",
        }
    }

    pub fn parse(s: &str) -> Option<Code> {
        match s {
            "SYSTEM" => Some(Code::System),
            "INTERFACE" => Some(Code::Interface),
            "INVARIANT" => Some(Code::Invariant),
            "OPERATIONAL" => Some(Code::Operational),
            _ => None,
        }
    }
}

pub const DIAG_FORBIDDEN_CHARS: [char; 3] = [';', ':', '\u{2014}'];

pub fn contains_cli_invocation(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut i = 0usize;
    while let Some(pos) = text[i..].find("grove") {
        let start = i + pos;
        let end = start + "grove".len();
        let after_ok = end >= bytes.len() || !bytes[end].is_ascii_alphanumeric();
        let before_ok =
            start == 0 || !(bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'-');
        if after_ok && before_ok {
            let mut j = end;
            while j < bytes.len()
                && (bytes[j] == b'`' || bytes[j] == b'"' || bytes[j].is_ascii_whitespace())
            {
                j += 1;
            }
            let tok_start = j;
            while j < bytes.len()
                && (bytes[j].is_ascii_lowercase() || bytes[j] == b'_' || bytes[j] == b'-')
            {
                j += 1;
            }
            if j > tok_start {
                let token = &text[tok_start..j];
                if crate::cli::COMMAND_NAMES.contains(&token) {
                    return true;
                }
            }
        }
        i = start + 1;
    }
    false
}

pub fn validate_diag_text(text: &str) -> Result<(), String> {
    if text.trim().is_empty() {
        return Err("text is empty".to_string());
    }
    for c in text.chars() {
        if DIAG_FORBIDDEN_CHARS.contains(&c) {
            return Err(format!(
                "text contains forbidden character {:?}",
                c.escape_default().to_string()
            ));
        }
    }
    if contains_cli_invocation(text) {
        return Err("text invokes a CLI command".to_string());
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: Code,
    pub text: String,
    pub details: Vec<(String, String)>,
}

impl Diagnostic {
    pub fn new(severity: Severity, code: Code, text: &str) -> Result<Diagnostic, String> {
        Diagnostic::with_details(severity, code, text, Vec::new())
    }

    pub fn with_details(
        severity: Severity,
        code: Code,
        text: &str,
        details: Vec<(String, String)>,
    ) -> Result<Diagnostic, String> {
        validate_diag_text(text)?;
        Ok(Diagnostic {
            severity,
            code,
            text: text.to_string(),
            details,
        })
    }

    pub fn error(code: Code, text: &str) -> Diagnostic {
        Diagnostic::new(Severity::Error, code, text).expect("authored diagnostic text is valid")
    }

    pub fn warning(code: Code, text: &str) -> Diagnostic {
        Diagnostic::new(Severity::Warning, code, text).expect("authored diagnostic text is valid")
    }

    pub fn info(code: Code, text: &str) -> Diagnostic {
        Diagnostic::new(Severity::Info, code, text).expect("authored diagnostic text is valid")
    }

    pub fn error_details(code: Code, text: &str, details: Vec<(String, String)>) -> Diagnostic {
        Diagnostic::with_details(Severity::Error, code, text, details)
            .expect("authored diagnostic text is valid")
    }

    pub fn warning_details(code: Code, text: &str, details: Vec<(String, String)>) -> Diagnostic {
        Diagnostic::with_details(Severity::Warning, code, text, details)
            .expect("authored diagnostic text is valid")
    }

    pub fn info_details(code: Code, text: &str, details: Vec<(String, String)>) -> Diagnostic {
        Diagnostic::with_details(Severity::Info, code, text, details)
            .expect("authored diagnostic text is valid")
    }

    pub fn system_error(text: &str) -> Diagnostic {
        Diagnostic::error(Code::System, text)
    }

    pub fn system_error_details(text: &str, details: Vec<(String, String)>) -> Diagnostic {
        Diagnostic::error_details(Code::System, text, details)
    }

    pub fn system_warning(text: &str) -> Diagnostic {
        Diagnostic::warning(Code::System, text)
    }

    pub fn system_warning_details(text: &str, details: Vec<(String, String)>) -> Diagnostic {
        Diagnostic::warning_details(Code::System, text, details)
    }

    pub fn interface_error(text: &str) -> Diagnostic {
        Diagnostic::error(Code::Interface, text)
    }

    pub fn interface_error_details(text: &str, details: Vec<(String, String)>) -> Diagnostic {
        Diagnostic::error_details(Code::Interface, text, details)
    }

    pub fn invariant_error(text: &str) -> Diagnostic {
        Diagnostic::error(Code::Invariant, text)
    }

    pub fn invariant_error_details(text: &str, details: Vec<(String, String)>) -> Diagnostic {
        Diagnostic::error_details(Code::Invariant, text, details)
    }

    pub fn operational_info(text: &str) -> Diagnostic {
        Diagnostic::info(Code::Operational, text)
    }

    pub fn operational_info_details(text: &str, details: Vec<(String, String)>) -> Diagnostic {
        Diagnostic::info_details(Code::Operational, text, details)
    }

    pub fn operational_warning(text: &str) -> Diagnostic {
        Diagnostic::warning(Code::Operational, text)
    }

    pub fn operational_warning_details(text: &str, details: Vec<(String, String)>) -> Diagnostic {
        Diagnostic::warning_details(Code::Operational, text, details)
    }

    pub fn raw(severity: Severity, code: Code, text: &str) -> Diagnostic {
        Diagnostic {
            severity,
            code,
            text: text.to_string(),
            details: Vec::new(),
        }
    }

    pub fn render(&self) -> String {
        let mut s = format!(
            "{} {} {}",
            self.severity.as_str(),
            self.code.as_str(),
            self.text
        );
        for (k, v) in &self.details {
            s.push_str("\n  ");
            s.push_str(k);
            s.push('=');
            s.push_str(v);
        }
        s
    }

    pub fn to_jval(&self) -> JVal {
        let mut obj = JuliaDict::from_pairs(vec![
            (
                "severity".to_string(),
                JVal::Str(self.severity.as_str().to_string()),
            ),
            (
                "code".to_string(),
                JVal::Str(self.code.as_str().to_string()),
            ),
            ("text".to_string(), JVal::Str(self.text.clone())),
        ]);
        if !self.details.is_empty() {
            let pairs = self
                .details
                .iter()
                .map(|(k, v)| (k.clone(), JVal::Str(v.clone())))
                .collect();
            obj.insert(
                "details".to_string(),
                JVal::Obj(JuliaDict::from_pairs(pairs)),
            );
        }
        JVal::Obj(obj)
    }

    pub fn to_json(&self) -> String {
        emit_jval(&self.to_jval())
    }
}
