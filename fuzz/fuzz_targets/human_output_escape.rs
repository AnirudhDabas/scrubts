#![no_main]

use libfuzzer_sys::fuzz_target;
use scrub_report::{human_safe, is_forbidden_human_control};

fuzz_target!(|input: &[u8]| {
    let input = String::from_utf8_lossy(input);
    let rendered = human_safe(&input);
    assert!(!rendered.chars().any(is_forbidden_human_control));
    assert_eq!(human_safe(&rendered), rendered);
    if !input.chars().any(is_forbidden_human_control) {
        assert_eq!(rendered, input);
    }
});
