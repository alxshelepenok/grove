use crate::model::{Edge, Node};
use std::time::{SystemTime, UNIX_EPOCH};

thread_local! {
    static CLOCK_UNIX_OVERRIDE: std::cell::Cell<Option<i64>> = const { std::cell::Cell::new(None) };
}

pub fn set_clock_unix_override(v: Option<i64>) {
    CLOCK_UNIX_OVERRIDE.with(|c| c.set(v));
}

pub fn unix_now() -> i64 {
    if let Some(v) = CLOCK_UNIX_OVERRIDE.with(|c| c.get()) {
        return v;
    }
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

pub fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 } as i64;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

pub fn format_unix_utc(secs: i64) -> String {
    let days = secs.div_euclid(86400);
    let rem = secs.rem_euclid(86400);
    let (y, m, d) = civil_from_days(days);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y,
        m,
        d,
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

pub fn utc_stamp_second() -> String {
    format_unix_utc(unix_now())
}

pub fn parse_rfc3339_utc_second(s: &str) -> Option<i64> {
    let s = s.trim();
    let b = s.as_bytes();
    if b.len() != 20 || b[19] != b'Z' {
        return None;
    }
    if b[4] != b'-' || b[7] != b'-' || b[10] != b'T' || b[13] != b':' || b[16] != b':' {
        return None;
    }
    let num = |lo: usize, hi: usize| -> Option<i64> {
        s[lo..hi].parse().ok()
    };
    let y = num(0, 4)?;
    let mo = num(5, 7)? as u32;
    let d = num(8, 10)? as u32;
    let h = num(11, 13)?;
    let mi = num(14, 16)?;
    let sec = num(17, 19)?;
    if !(1..=12).contains(&mo) || !(1..=31).contains(&d) {
        return None;
    }
    Some(days_from_civil(y, mo, d) * 86400 + h * 3600 + mi * 60 + sec)
}

fn timestamp_blank(x: &str) -> bool {
    x.trim().is_empty()
}

pub fn stamp_new_node(n: &mut Node) {
    let t = utc_stamp_second();
    n.attrs.insert("t_created".to_string(), t.clone());
    n.attrs.insert("t_updated".to_string(), t);
}

pub fn stamp_touch_node(n: &mut Node) {
    let t = utc_stamp_second();
    n.attrs.insert("t_updated".to_string(), t.clone());
    let created = n.attr("t_created");
    if timestamp_blank(&created) {
        n.attrs.insert("t_created".to_string(), t);
    }
}

pub fn stamp_new_edge(e: &mut Edge) {
    let blank = match &e.t_created {
        None => true,
        Some(t) => timestamp_blank(t),
    };
    if blank {
        e.t_created = Some(utc_stamp_second());
    }
}
