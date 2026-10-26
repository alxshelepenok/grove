mod common;
#[allow(unused_imports)]
use common::*;
use grove_core::*;

#[test]
fn code_roundtrip_covers_all_four_variants() {
    assert_eq!(Code::ALL.len(), 4);
    assert_eq!(Code::System.as_str(), "SYSTEM");
    assert_eq!(Code::Interface.as_str(), "INTERFACE");
    assert_eq!(Code::Invariant.as_str(), "INVARIANT");
    assert_eq!(Code::Operational.as_str(), "OPERATIONAL");
    for c in Code::ALL {
        assert_eq!(Code::parse(c.as_str()), Some(c));
    }
}

#[test]
fn code_parse_rejects_legacy_and_invalid_values() {
    for bad in [
        "",
        "SYSTEM ",
        "system",
        "General",
        "Other",
        "Info",
        "Err",
        "Usage",
        "Guard",
        "NotFound",
        "Checksum",
        "Io",
        "NOT_FOUND",
        "ERR",
        "USAGE",
        "NODE_UNKNOWN",
        "I9_ASSUMPTION_CHAIN",
        "LOCK_MISSING",
        "CHECK_OK",
        "UNDO_NO_JOURNAL",
        "REVALIDATE_STATUS",
        "DECAY_ORIGIN_ROTTED",
        "SESSION_LOCK_TIMEOUT",
        "GOAL_VERIFIED_DISTILL",
        "UNKNOWN_COMMAND",
        "DOR_NOT_SATISFIED",
        "I5_PREDECESSORS",
    ] {
        assert_eq!(
            Code::parse(bad),
            None,
            "legacy or invalid code {bad} must not parse"
        );
    }
}

#[test]
fn semantic_constructors_fix_coherent_severity_pairs() {
    let d = Diagnostic::system_error("disk failure");
    assert_eq!((d.severity, d.code), (Severity::Error, Code::System));
    let d = Diagnostic::system_warning("registry malformed");
    assert_eq!((d.severity, d.code), (Severity::Warning, Code::System));
    let d = Diagnostic::interface_error("unknown command");
    assert_eq!((d.severity, d.code), (Severity::Error, Code::Interface));
    let d = Diagnostic::invariant_error("terminal record is frozen");
    assert_eq!((d.severity, d.code), (Severity::Error, Code::Invariant));
    let d = Diagnostic::operational_info("initialised");
    assert_eq!((d.severity, d.code), (Severity::Info, Code::Operational));
    let d = Diagnostic::operational_warning("nothing to undo");
    assert_eq!((d.severity, d.code), (Severity::Warning, Code::Operational));
}

#[test]
fn generic_constructors_accept_every_code_with_every_severity() {
    for c in Code::ALL {
        assert_eq!(Diagnostic::error(c, "text").severity, Severity::Error);
        assert_eq!(Diagnostic::warning(c, "text").severity, Severity::Warning);
        assert_eq!(Diagnostic::info(c, "text").severity, Severity::Info);
    }
}

#[test]
fn new_rejects_each_forbidden_character() {
    for bad in [';', ':', '\u{2014}'] {
        let text = format!("text with {} inside", bad);
        let err =
            Diagnostic::new(Severity::Error, Code::Interface, &text).expect_err("must reject");
        assert!(err.contains("forbidden character"), "{err}");
    }
    assert!(Diagnostic::new(Severity::Error, Code::Interface, "clean text").is_ok());
}

#[test]
fn new_rejects_empty_text() {
    let err = Diagnostic::new(Severity::Error, Code::Interface, "   ").expect_err("must reject");
    assert!(err.contains("empty"), "{err}");
}

#[test]
fn new_rejects_cli_invocation_guidance() {
    for text in [
        "run `grove undo` to recover",
        "see grove dor W-01 for details",
        "inspect with grove check first",
        "grove init the project",
    ] {
        let err = Diagnostic::new(Severity::Error, Code::Interface, text).expect_err("must reject");
        assert!(err.contains("CLI command"), "{err} for {text}");
    }
    let ok = Diagnostic::new(
        Severity::Error,
        Code::Interface,
        "the Grove skill documents the Delivery loop",
    );
    assert!(ok.is_ok());
}

#[test]
fn severity_parse_roundtrip() {
    assert_eq!(Severity::parse("error"), Some(Severity::Error));
    assert_eq!(Severity::parse("warning"), Some(Severity::Warning));
    assert_eq!(Severity::parse("info"), Some(Severity::Info));
    assert_eq!(Severity::parse("fatal"), None);
    assert_eq!(Severity::Error.as_str(), "error");
    assert_eq!(Severity::Warning.as_str(), "warning");
    assert_eq!(Severity::Info.as_str(), "info");
}

#[test]
fn render_is_deterministic_severity_code_text() {
    let d = Diagnostic::invariant_error("node is terminal");
    assert_eq!(d.render(), "error INVARIANT node is terminal");
    let w = Diagnostic::system_warning("careful");
    assert_eq!(w.render(), "warning SYSTEM careful");
    let i = Diagnostic::operational_info("note");
    assert_eq!(i.render(), "info OPERATIONAL note");
}

#[test]
fn details_hold_paths_and_dynamic_values_and_render_indented() {
    let d = Diagnostic::system_error_details(
        "lock file is absent, initialize the project first",
        vec![
            (
                "path".to_string(),
                "Y:/tmp/proj/.grove/state.lock".to_string(),
            ),
            ("attempt".to_string(), "2".to_string()),
        ],
    );
    let rendered = d.render();
    assert_eq!(
        rendered,
        "error SYSTEM lock file is absent, initialize the project first\n  path=Y:/tmp/proj/.grove/state.lock\n  attempt=2"
    );

    assert!(d.details[0].1.contains(".grove"));
}

#[test]
fn json_conversion_uses_severity_code_text_and_optional_details() {
    let plain = Diagnostic::interface_error("plain message");
    assert_eq!(
        plain.to_json(),
        "{\"severity\":\"error\",\"code\":\"INTERFACE\",\"text\":\"plain message\"}"
    );
    let detailed = Diagnostic::operational_warning_details(
        "with details",
        vec![("path".to_string(), "a/b.txt".to_string())],
    );
    let v = parse_json(&detailed.to_json()).expect("parses");
    assert_eq!(v.get("severity").and_then(|x| x.as_str()), Some("warning"));
    assert_eq!(v.get("code").and_then(|x| x.as_str()), Some("OPERATIONAL"));
    assert_eq!(v.get("text").and_then(|x| x.as_str()), Some("with details"));
    let details = v.get("details").expect("details present");
    let obj = details.as_obj().expect("details object");
    assert!(obj.iter().any(|(k, _)| k == "path"));
}

#[test]
fn exit_codes_stay_independent_from_code() {
    let by_exit = [
        (1, Code::Interface),
        (1, Code::System),
        (2, Code::System),
        (3, Code::Invariant),
        (4, Code::Invariant),
        (5, Code::Interface),
    ];
    for (exit, c) in by_exit {
        let r = OpResult::fail(exit, c, "same text");
        assert_eq!(r.code, exit);
        assert_eq!(r.diags[0].code, c);
        assert_eq!(
            r.diags[0].render(),
            format!("error {} same text", c.as_str())
        );
    }
}

#[test]
fn op_result_renders_canonical_grammar_for_failures_and_notices() {
    let mut ok = OpResult::ok();
    ok.push_info(Code::Operational, "all done");
    ok.push_warning(Code::System, "mind the gap");
    assert_eq!(
        ok.err_rendered(false),
        "info OPERATIONAL all done\nwarning SYSTEM mind the gap\n"
    );
    let quiet = ok.err_rendered(true);
    assert_eq!(quiet, "warning SYSTEM mind the gap\n");
    let failed = OpResult::fail(4, Code::Invariant, "guard said no");
    assert_eq!(
        failed.err_rendered(false),
        "error INVARIANT guard said no\n"
    );
    let structured = OpResult::fail_diag(
        5,
        Diagnostic::interface_error_details(
            "not found W-99",
            vec![("id".to_string(), "W-99".to_string())],
        ),
    );
    assert_eq!(
        structured.err_rendered(false),
        "error INTERFACE not found W-99\n  id=W-99\n"
    );
}

#[test]
fn conformance_validators_accept_only_four_categories() {
    for code in Code::ALL {
        common::assert_rendered_diag_codes(&format!("error {} message\n", code.as_str()));
        let json =
            serde_json::json!({"severity": "error", "code": code.as_str(), "text": "message"});
        common::assert_json_diag_codes(&json);
    }
    for legacy in [
        "GENERAL",
        "OTHER",
        "INFO",
        "ERR",
        "NOT_FOUND",
        "DOR_NOT_SATISFIED",
    ] {
        let rendered = format!("error {legacy} message\n");
        assert!(
            std::panic::catch_unwind(|| { common::assert_rendered_diag_codes(&rendered) }).is_err()
        );
        let json = serde_json::json!({"severity": "error", "code": legacy, "text": "message"});
        assert!(std::panic::catch_unwind(|| { common::assert_json_diag_codes(&json) }).is_err());
    }
    common::assert_rendered_diag_codes("grove help and documents: remain unchanged\n");
    common::assert_json_output_diag_codes("not JSON");
}
