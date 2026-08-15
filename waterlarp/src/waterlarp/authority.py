"""Authority metadata for exact runnable configurations and non-runnable provider slots."""

from __future__ import annotations

from dataclasses import asdict, dataclass
from enum import StrEnum
from typing import Any


class AuthorityClass(StrEnum):
    PUBLIC_REFERENCE = "PUBLIC_REFERENCE"
    PUBLIC_MECHANISM_PRIVATE_KEY = "PUBLIC_MECHANISM_PRIVATE_KEY"
    PROVIDER_GATED = "PROVIDER_GATED"
    UNDOCUMENTED_PROVIDER = "UNDOCUMENTED_PROVIDER"
    THIRD_PARTY_REIMPLEMENTATION = "THIRD_PARTY_REIMPLEMENTATION"


class GuaranteeClass(StrEnum):
    DISTORTIONARY = "DISTORTIONARY"
    DISTRIBUTION_PRESERVING_IN_EXPECTATION = "DISTRIBUTION_PRESERVING_IN_EXPECTATION"
    UNBIASED = "UNBIASED"
    NONE_CLAIMED = "NONE_CLAIMED"
    UNDOCUMENTED = "UNDOCUMENTED"


class DetectorAvailability(StrEnum):
    AVAILABLE = "AVAILABLE"
    ANNOUNCED_FORTHCOMING = "ANNOUNCED_FORTHCOMING"
    UNAVAILABLE = "UNAVAILABLE"


class DeploymentConfigurationStatus(StrEnum):
    UNDISCLOSED = "UNDISCLOSED"


class ProviderKeyStatus(StrEnum):
    PRIVATE_NOT_PUBLIC = "PRIVATE_NOT_PUBLIC"


class ProviderApiContractStatus(StrEnum):
    UNKNOWN_NOT_PUBLIC = "UNKNOWN_NOT_PUBLIC"


class ProviderParityStatus(StrEnum):
    NOT_ESTABLISHED = "NOT_ESTABLISHED"


class PublicReferenceRelationship(StrEnum):
    RELATED_FAMILY_NOT_DEPLOYMENT_PARITY = "RELATED_FAMILY_NOT_DEPLOYMENT_PARITY"


@dataclass(frozen=True)
class ProviderDeploymentAuthority:
    provider: str
    publicly_disclosed_mechanism_family: str
    mechanism_family_authority_source_ids: tuple[str, ...]
    exact_deployed_configuration: DeploymentConfigurationStatus
    provider_key: ProviderKeyStatus
    provider_detector_api_contract: ProviderApiContractStatus
    exact_public_provider_detector_parity: ProviderParityStatus
    public_reference_family: str
    public_reference_relationship: PublicReferenceRelationship
    runnable_exact_provider_detector: bool

    def validate(self) -> None:
        if not self.provider or not self.publicly_disclosed_mechanism_family:
            raise ValueError("provider and publicly disclosed mechanism family are required")
        if not self.mechanism_family_authority_source_ids:
            raise ValueError("mechanism-family authority sources must not be empty")
        if not self.public_reference_family:
            raise ValueError("a named public reference family is required")
        if self.runnable_exact_provider_detector:
            raise ValueError("this provider-deployment state does not authorize exact detection")


@dataclass(frozen=True)
class AuthorityRecord:
    mechanism_name: str
    mechanism_version: str
    mechanism_authority: AuthorityClass
    implementation_authority: AuthorityClass
    detector_authority: AuthorityClass
    key_provenance: str
    threshold_provenance: str
    detector_training_requirement: str
    model_logit_requirement: bool
    guarantee_class: GuaranteeClass
    authority_source_ids: tuple[str, ...]
    detector_available: bool
    detector_availability: DetectorAvailability
    runnable: bool
    provider_deployment: ProviderDeploymentAuthority | None
    limitations: tuple[str, ...]

    def validate(self) -> None:
        if not self.authority_source_ids:
            raise ValueError("authority_source_ids must not be empty")
        expected_available = self.detector_availability is DetectorAvailability.AVAILABLE
        if self.detector_available != expected_available:
            raise ValueError("detector_available must agree with detector_availability")
        if self.runnable and not expected_available:
            raise ValueError("a runnable mechanism must have an available detector")
        if self.mechanism_authority is AuthorityClass.UNDOCUMENTED_PROVIDER and self.runnable:
            raise ValueError("an undocumented provider mechanism cannot be runnable")
        if self.provider_deployment is not None:
            self.provider_deployment.validate()
            if not set(self.provider_deployment.mechanism_family_authority_source_ids) <= set(
                self.authority_source_ids
            ):
                raise ValueError("mechanism-family sources must belong to the authority record")
            if self.mechanism_authority is not AuthorityClass.PUBLIC_MECHANISM_PRIVATE_KEY:
                raise ValueError(
                    "a documented private-key provider family needs PUBLIC_MECHANISM_PRIVATE_KEY"
                )
            if self.provider_deployment.public_reference_family == self.mechanism_name:
                raise ValueError(
                    "a related public reference must have a distinct authority identity"
                )
            if self.provider_deployment.runnable_exact_provider_detector != self.runnable:
                raise ValueError(
                    "provider detector runnable state must agree with the authority record"
                )
        if not self.limitations:
            raise ValueError("limitations must be stated beside capabilities")

    def require_runnable(self) -> None:
        self.validate()
        if not self.runnable:
            raise UnsupportedMechanismError(
                f"{self.mechanism_name} is not runnable under its frozen authority record"
            )

    def to_dict(self) -> dict[str, Any]:
        self.validate()
        return asdict(self)


class UnsupportedMechanismError(RuntimeError):
    """Raised when a configuration asks WaterLARP to invent unsupported behavior."""


KGW_AUTHORITY = AuthorityRecord(
    mechanism_name="reference.kgw",
    mechanism_version="lm-watermarking@82922516930c02f8aa322765defdb5863d07a00e",
    mechanism_authority=AuthorityClass.PUBLIC_REFERENCE,
    implementation_authority=AuthorityClass.PUBLIC_REFERENCE,
    detector_authority=AuthorityClass.PUBLIC_REFERENCE,
    key_provenance="WaterLARP deterministic benchmark key; not the published demo key",
    threshold_provenance="calibration split or explicitly named analytical null",
    detector_training_requirement="none",
    model_logit_requirement=True,
    guarantee_class=GuaranteeClass.DISTORTIONARY,
    authority_source_ids=("kgw", "kgw-reliability"),
    detector_available=True,
    detector_availability=DetectorAvailability.AVAILABLE,
    runnable=True,
    provider_deployment=None,
    limitations=(
        "The result supports only reference KGW semantics under the recorded configuration.",
        "Repeated n-grams are counted once; CPU and CUDA RNG streams are not interchangeable.",
    ),
)

SYNTHID_AUTHORITY = AuthorityRecord(
    mechanism_name="reference.synthid_text",
    mechanism_version="transformers-v5.15.0@5eddc12edfaf8cafde8c9bae4ccb12f8a139b4f9",
    mechanism_authority=AuthorityClass.PUBLIC_REFERENCE,
    implementation_authority=AuthorityClass.PUBLIC_REFERENCE,
    detector_authority=AuthorityClass.PUBLIC_REFERENCE,
    key_provenance="WaterLARP deterministic benchmark keys; not a Gemini deployment key",
    threshold_provenance="length-specific clean calibration",
    detector_training_requirement="none for Weighted Mean; per-key training for Bayesian",
    model_logit_requirement=True,
    guarantee_class=GuaranteeClass.DISTRIBUTION_PRESERVING_IN_EXPECTATION,
    authority_source_ids=("synthid-text", "synthid-text-transformers"),
    detector_available=True,
    detector_availability=DetectorAvailability.AVAILABLE,
    runnable=True,
    provider_deployment=None,
    limitations=(
        "Reference hashing differs from the Gemini App according to official Transformers source.",
        "Weighted Mean thresholds depend on text length; Bayesian detection is secondary.",
    ),
)

ANTHROPIC_AUTHORITY = AuthorityRecord(
    mechanism_name="anthropic.embedded_text_watermark",
    mechanism_version=(
        "provider documentation published 2026-08-14; exact deployment configuration undisclosed"
    ),
    mechanism_authority=AuthorityClass.PUBLIC_MECHANISM_PRIVATE_KEY,
    implementation_authority=AuthorityClass.UNDOCUMENTED_PROVIDER,
    detector_authority=AuthorityClass.UNDOCUMENTED_PROVIDER,
    key_provenance="Anthropic provider key; private and not public",
    threshold_provenance="unknown",
    detector_training_requirement="unknown",
    model_logit_requirement=False,
    guarantee_class=GuaranteeClass.NONE_CLAIMED,
    authority_source_ids=("anthropic-claude-text-watermark", "anthropic-claude-marking"),
    detector_available=False,
    detector_availability=DetectorAvailability.ANNOUNCED_FORTHCOMING,
    runnable=False,
    provider_deployment=ProviderDeploymentAuthority(
        provider="Anthropic",
        publicly_disclosed_mechanism_family="version of the SynthID-Text approach",
        mechanism_family_authority_source_ids=("anthropic-claude-text-watermark",),
        exact_deployed_configuration=DeploymentConfigurationStatus.UNDISCLOSED,
        provider_key=ProviderKeyStatus.PRIVATE_NOT_PUBLIC,
        provider_detector_api_contract=ProviderApiContractStatus.UNKNOWN_NOT_PUBLIC,
        exact_public_provider_detector_parity=ProviderParityStatus.NOT_ESTABLISHED,
        public_reference_family="reference.synthid_text",
        public_reference_relationship=(
            PublicReferenceRelationship.RELATED_FAMILY_NOT_DEPLOYMENT_PARITY
        ),
        runnable_exact_provider_detector=False,
    ),
    limitations=(
        "Anthropic identifies the mechanism family, not Claude's exact deployed configuration.",
        "Anthropic's detection API is announced but is not currently public or runnable here.",
        "A public SynthID reference result is not an Anthropic provider-detector result.",
        "Exact Claude watermark status remains UNKNOWN unless an authoritative detector executes.",
    ),
)

AUTHORITY_RECORDS = {
    record.mechanism_name: record
    for record in (KGW_AUTHORITY, SYNTHID_AUTHORITY, ANTHROPIC_AUTHORITY)
}


def authority_for(mechanism_name: str, *, require_runnable: bool = False) -> AuthorityRecord:
    try:
        record = AUTHORITY_RECORDS[mechanism_name]
    except KeyError as exc:
        raise UnsupportedMechanismError(f"unknown mechanism: {mechanism_name}") from exc
    record.validate()
    if require_runnable:
        record.require_runnable()
    return record
