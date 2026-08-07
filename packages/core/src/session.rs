use crate::model::{Kind, Node};
use crate::times::{parse_rfc3339_utc_second, unix_now, utc_stamp_second};
use sha2::{Digest, Sha256};

pub const SESSION_DISPLAY_STALE_AFTER_HOURS: i64 = 24;

pub fn progress_has_session_record(w: &Node) -> bool {
    match w.attrs.get("session") {
        Some(v) => !v.trim().is_empty(),
        None => false,
    }
}

pub fn session_token_matches(w: &Node, eff: &str) -> bool {
    if !progress_has_session_record(w) {
        return false;
    }
    w.attrs.get("session").map(|s| s.trim()) == Some(eff.trim())
}

pub fn session_claim_age_stale(w: &Node) -> bool {
    session_claim_age_stale_at(w, unix_now())
}

pub fn session_claim_age_stale_at(w: &Node, now_unix: i64) -> bool {
    let sa = w.attr("session_at");
    let ts = match parse_rfc3339_utc_second(&sa) {
        Some(ts) => ts,
        None => return false,
    };
    now_unix - ts > SESSION_DISPLAY_STALE_AFTER_HOURS * 3600
}

pub fn progress_session_display_stale(w: &Node, eff: &str) -> bool {
    if w.status != "progress" {
        return false;
    }
    if !progress_has_session_record(w) {
        return true;
    }
    if !session_token_matches(w, eff) {
        return true;
    }
    session_claim_age_stale(w)
}

pub fn session_release_denied_message(w: &Node) -> String {
    format!(
        "I11/session: cannot release {}: token differs and claim is fresh (<{}h); pass the owning GROVE_SESSION/--session, use `grove resume`, or wait",
        w.id, SESSION_DISPLAY_STALE_AFTER_HOURS
    )
}

pub fn session_mutate_denied_message(w: &Node) -> String {
    format!(
        "I11/session: {} is `progress` and owned by another session; try `grove resume {}` after adopting, or coordinate a `grove handoff`",
        w.id, w.id
    )
}

pub fn session_denial_progress_mutate(w: &Node, eff: &str) -> Option<String> {
    if w.kind != Kind::W {
        return None;
    }
    if w.status != "progress" {
        return None;
    }
    if !progress_has_session_record(w) {
        return None;
    }
    if session_token_matches(w, eff) {
        return None;
    }
    Some(session_mutate_denied_message(w))
}

pub fn session_denial_progress_release(w: &Node, eff: &str) -> Option<String> {
    if w.kind != Kind::W {
        return None;
    }
    if w.status != "progress" {
        return None;
    }
    if !progress_has_session_record(w) {
        return None;
    }
    if session_token_matches(w, eff) {
        return None;
    }
    if session_claim_age_stale(w) {
        return None;
    }
    Some(session_release_denied_message(w))
}

pub fn assign_w_claim_session(w: &mut Node, token: &str) {
    w.attrs
        .insert("session".to_string(), token.trim().to_string());
    w.attrs.insert("session_at".to_string(), utc_stamp_second());
}

pub fn clear_w_session_attrs(w: &mut Node) {
    w.attrs.remove("session");
    w.attrs.remove("session_at");
}

pub fn host_slug_for_session() -> String {
    for k in ["COMPUTERNAME", "HOSTNAME", "HOST"] {
        if let Ok(v) = std::env::var(k) {
            let v = v.trim();
            if !v.is_empty() {
                return v.to_lowercase();
            }
        }
    }
    "host".to_string()
}

fn sha256_hex16(data: &[u8]) -> String {
    let digest = Sha256::digest(data);
    let mut s = String::with_capacity(16);
    for b in &digest[..8] {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

pub fn derive_default_session_token(root: &str) -> String {
    let abs = std::path::absolute(root).unwrap_or_else(|_| std::path::PathBuf::from(root));
    let rp = abs.to_string_lossy().replace('\\', "/").to_lowercase();
    let dig = sha256_hex16(rp.as_bytes());
    format!("{}:{}", host_slug_for_session(), dig)
}

pub fn effective_session_token(root: &str, kw_session: Option<&str>) -> String {
    if let Some(t) = kw_session {
        let t = t.trim();
        if !t.is_empty() {
            return t.to_string();
        }
    }
    if let Ok(env) = std::env::var("GROVE_SESSION") {
        let env = env.trim();
        if !env.is_empty() {
            return env.to_string();
        }
    }
    derive_default_session_token(root)
}
