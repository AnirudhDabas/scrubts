import pytest

from waterlarp.authority import (
    ANTHROPIC_AUTHORITY,
    AUTHORITY_RECORDS,
    KGW_AUTHORITY,
    SYNTHID_AUTHORITY,
    AuthorityClass,
    DeploymentConfigurationStatus,
    DetectorAvailability,
    ProviderApiContractStatus,
    ProviderKeyStatus,
    ProviderParityStatus,
    PublicReferenceRelationship,
    UnsupportedMechanismError,
    authority_for,
)


def test_authority_records_validate() -> None:
    for record in AUTHORITY_RECORDS.values():
        record.validate()


def test_claude_slot_refuses_execution() -> None:
    with pytest.raises(UnsupportedMechanismError, match="not runnable"):
        ANTHROPIC_AUTHORITY.require_runnable()


def test_anthropic_family_is_documented_but_deployment_is_undisclosed() -> None:
    deployment = ANTHROPIC_AUTHORITY.provider_deployment
    assert ANTHROPIC_AUTHORITY.mechanism_authority is AuthorityClass.PUBLIC_MECHANISM_PRIVATE_KEY
    assert deployment is not None
    assert deployment.provider == "Anthropic"
    assert deployment.publicly_disclosed_mechanism_family == (
        "version of the SynthID-Text approach"
    )
    assert deployment.exact_deployed_configuration is DeploymentConfigurationStatus.UNDISCLOSED
    assert deployment.provider_key is ProviderKeyStatus.PRIVATE_NOT_PUBLIC


def test_announced_anthropic_detector_is_not_available_or_runnable() -> None:
    deployment = ANTHROPIC_AUTHORITY.provider_deployment
    assert deployment is not None
    assert ANTHROPIC_AUTHORITY.detector_availability is (DetectorAvailability.ANNOUNCED_FORTHCOMING)
    assert ANTHROPIC_AUTHORITY.detector_available is False
    assert ANTHROPIC_AUTHORITY.runnable is False
    assert deployment.runnable_exact_provider_detector is False
    assert deployment.provider_detector_api_contract is (
        ProviderApiContractStatus.UNKNOWN_NOT_PUBLIC
    )


def test_reference_synthid_cannot_satisfy_anthropic_provider_authority() -> None:
    deployment = ANTHROPIC_AUTHORITY.provider_deployment
    assert deployment is not None
    assert deployment.public_reference_family == SYNTHID_AUTHORITY.mechanism_name
    assert deployment.public_reference_relationship is (
        PublicReferenceRelationship.RELATED_FAMILY_NOT_DEPLOYMENT_PARITY
    )
    assert deployment.exact_public_provider_detector_parity is (
        ProviderParityStatus.NOT_ESTABLISHED
    )
    assert SYNTHID_AUTHORITY.mechanism_name != ANTHROPIC_AUTHORITY.mechanism_name
    assert authority_for(ANTHROPIC_AUTHORITY.mechanism_name) is ANTHROPIC_AUTHORITY
    assert authority_for(SYNTHID_AUTHORITY.mechanism_name) is SYNTHID_AUTHORITY


@pytest.mark.parametrize("reference_decision", (False, True))
def test_reference_synthid_decision_cannot_answer_claude_status(
    reference_decision: bool,
) -> None:
    assert isinstance(reference_decision, bool)
    with pytest.raises(UnsupportedMechanismError, match="not runnable"):
        authority_for(ANTHROPIC_AUTHORITY.mechanism_name, require_runnable=True)


def test_public_reference_authorities_are_semantically_unchanged() -> None:
    for record in (KGW_AUTHORITY, SYNTHID_AUTHORITY):
        assert record.mechanism_authority is AuthorityClass.PUBLIC_REFERENCE
        assert record.detector_availability is DetectorAvailability.AVAILABLE
        assert record.detector_available is True
        assert record.runnable is True
        assert record.provider_deployment is None
