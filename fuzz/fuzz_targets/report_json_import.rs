#![no_main]

use std::io::Cursor;
use std::path::Path;
use std::sync::OnceLock;

use libfuzzer_sys::fuzz_target;
use scrub::inspect_reader;
use scrub_report::Report;

fn baseline() -> &'static str {
    static BASELINE: OnceLock<String> = OnceLock::new();
    BASELINE.get_or_init(|| {
        inspect_reader(Path::new("structured.json"), &mut Cursor::new(b"seed"))
            .expect("fixed baseline inspection")
            .to_json()
            .expect("fixed baseline JSON")
    })
}

fn coherent_round_trip(input: &str) {
    if let Ok(untrusted) = Report::from_json(input) {
        let first = untrusted
            .as_report()
            .canonical_report_bytes()
            .expect("canonical imported report");
        let serialized = untrusted.as_report().to_json().expect("serialize report");
        let reparsed = Report::from_json(&serialized).expect("serialized report reparses");
        assert_eq!(
            first,
            reparsed
                .as_report()
                .canonical_report_bytes()
                .expect("canonical reparsed report")
        );
    }
}

fuzz_target!(|input: &[u8]| {
    coherent_round_trip(&String::from_utf8_lossy(input));

    let mut structured: serde_json::Value =
        serde_json::from_str(baseline()).expect("baseline is JSON");
    structured["artifact"]["path"] = serde_json::Value::String(
        String::from_utf8_lossy(input)
            .chars()
            .take(65_536)
            .collect(),
    );
    coherent_round_trip(&serde_json::to_string(&structured).expect("structured report JSON"));

    structured["findings"][0]["trace"]["verifier"]["id"] =
        serde_json::Value::String(format!("ontology-invalid-{}", input.len()));
    let invalid = serde_json::to_string(&structured).expect("invalid report JSON");
    assert!(Report::from_json(&invalid).is_err());
});
