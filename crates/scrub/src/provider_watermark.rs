use scrub_report::{Evidence, Finding, FindingStatus, MechanismIdentity};

pub(crate) const MECHANISM_ID: &str = "anthropic.embedded_text_watermark";
const VERSION: &str = "provider documentation published 2026-08-14; exact deployment undisclosed";

pub(crate) fn finding(textual_artifact: bool) -> Finding {
    let (status, evidence, limitations) = if textual_artifact {
        (
            FindingStatus::Unknown,
            vec![
                Evidence::new("detector_availability", "announced_forthcoming"),
                Evidence::new("exact_deployed_configuration", "undisclosed"),
                Evidence::new("provider_detector_api_contract", "unknown_not_public"),
                Evidence::new("provider_key", "private_not_public"),
                Evidence::new(
                    "public_reference_relationship",
                    "related_family_not_deployment_parity",
                ),
            ],
            vec![
                "The checked Anthropic authority snapshot records the exact deployed configuration, key, detector statistic, threshold, and API contract as unavailable to scrub.".to_owned(),
                "A public SynthID reference result is not an Anthropic provider-detector result; UNKNOWN does not mean human-written or unwatermarked.".to_owned(),
            ],
        )
    } else {
        (
            FindingStatus::NotApplicable,
            vec![],
            vec![
                "Claude embedded text-watermark inspection applies to text, not this supported binary artifact.".to_owned(),
            ],
        )
    };

    Finding::new(
        MechanismIdentity::new(MECHANISM_ID, VERSION),
        status,
        evidence,
        limitations,
        vec![],
    )
    .expect("frozen provider mechanism/status pair is valid")
}
