use grove_core::{parse, serialize, ParseMode};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

fn corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../grove/conformance/corpus")
}

fn mask_checksum(text: &str) -> String {
    const PREFIX: &str = "# checksum: sha256:";
    let start = text.find(PREFIX).expect("checksum line") + PREFIX.len();
    format!("{}<sha>{}", &text[..start], &text[start + 64..])
}

fn corpus_locks() -> Vec<(String, usize, String)> {
    let mut paths: Vec<PathBuf> = fs::read_dir(corpus_dir())
        .expect("corpus dir")
        .map(|e| e.expect("dir entry").path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    paths.sort();
    let mut out = Vec::new();
    for path in paths {
        let fname = path.file_name().unwrap().to_string_lossy().into_owned();
        let text = fs::read_to_string(&path).expect("read corpus file");
        let v: Value = serde_json::from_str(&text).expect("corpus json");
        for (i, step) in v["steps"].as_array().expect("steps").iter().enumerate() {
            if let Some(lock) = step["lock"].as_str() {
                out.push((fname.clone(), i, lock.to_string()));
            }
        }
    }
    out
}

#[test]
fn corpus_snapshots_roundtrip_byte_identically() {
    let locks = corpus_locks();
    assert!(!locks.is_empty());
    let mut total = 0usize;
    let mut ok = 0usize;
    let mut rejections: Vec<(String, usize, String)> = Vec::new();
    let mut mismatches: Vec<(String, usize)> = Vec::new();
    for (fname, i, lock) in &locks {
        total += 1;
        assert!(
            parse(lock, ParseMode::Strict).is_err(),
            "strict mode accepted masked fixture {fname} step {i}"
        );
        match parse(lock, ParseMode::Fixture) {
            Err(e) => rejections.push((fname.clone(), *i, e.to_string())),
            Ok(state) => {
                let masked = mask_checksum(&serialize(&state));
                if masked == *lock {
                    ok += 1;
                } else {
                    mismatches.push((fname.clone(), *i));
                }
            }
        }
    }
    println!(
        "corpus roundtrip: {ok}/{total} snapshots byte-identical, {} rejected, {} non-canonical",
        rejections.len(),
        mismatches.len()
    );
    for (f, i, m) in &rejections {
        println!("  rejected {f} step {i}: {m}");
    }
    for (f, i) in &mismatches {
        println!("  non-canonical {f} step {i} (empty :archive section is dropped on serialize)");
    }

    let expected_rejections = [
        (
            "check-foreign-status.json",
            2usize,
            "lock parse error at line 5: a record status must be present (got 'archived')",
        ),
        (
            "check-foreign-status.json",
            3,
            "lock parse error at line 5: a record status must be present (got 'archived')",
        ),
        (
            "check-y-archive.json",
            2,
            "lock parse error at line 6: y record in :archive section",
        ),
        (
            "check-y-archive.json",
            3,
            "lock parse error at line 6: y record in :archive section",
        ),
    ];
    assert_eq!(
        rejections.len(),
        expected_rejections.len(),
        "unexpected rejection set: {rejections:?}"
    );
    for (f, i, msg) in expected_rejections {
        assert!(
            rejections
                .iter()
                .any(|(rf, ri, rm)| rf.as_str() == f && *ri == i && rm.as_str() == msg),
            "missing rejection {f} step {i}: {msg}"
        );
    }
    assert_eq!(
        mismatches,
        vec![("check-y-archive.json".to_string(), 1usize)],
        "unexpected non-canonical set"
    );
    assert_eq!(ok + rejections.len() + mismatches.len(), total);
}

#[test]
fn known_invalid_snapshots_fail_in_both_modes() {
    let locks = corpus_locks();
    let cases = [
        (
            "check-foreign-status.json",
            3usize,
            "lock parse error at line 5: a record status must be present (got 'archived')",
        ),
        (
            "check-y-archive.json",
            3,
            "lock parse error at line 6: y record in :archive section",
        ),
    ];
    for (f, i, msg) in cases {
        let lock = &locks
            .iter()
            .find(|(lf, li, _)| lf.as_str() == f && *li == i)
            .unwrap_or_else(|| panic!("snapshot {f} step {i}"))
            .2;
        let ef = parse(lock, ParseMode::Fixture).expect_err("fixture mode must reject");
        assert_eq!(ef.to_string(), msg, "fixture mode message for {f}");
        assert!(
            parse(lock, ParseMode::Strict).is_err(),
            "strict mode must reject {f} step {i}"
        );
    }
}
