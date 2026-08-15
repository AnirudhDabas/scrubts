import json
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator
from jsonschema.exceptions import ValidationError
from referencing import Registry, Resource
from test_manifests import complete_manifest

from waterlarp.authority import ANTHROPIC_AUTHORITY
from waterlarp.manifests import canonical_json_bytes

SCHEMA_ROOT = Path(__file__).resolve().parents[1] / "schemas"


def registry() -> Registry:
    result = Registry()
    for path in SCHEMA_ROOT.glob("*.json"):
        schema = json.loads(path.read_text(encoding="utf-8"))
        result = result.with_resource(schema["$id"], Resource.from_contents(schema))
    return result


def test_public_schemas_parse_and_manifest_references_resolve() -> None:
    authority = {
        "mechanism_name": "fixture",
        "mechanism_version": "1",
        "mechanism_authority": "PUBLIC_REFERENCE",
        "implementation_authority": "PUBLIC_REFERENCE",
        "detector_authority": "PUBLIC_REFERENCE",
        "key_provenance": "fixture",
        "threshold_provenance": "fixture",
        "detector_training_requirement": "none",
        "model_logit_requirement": False,
        "guarantee_class": "NONE_CLAIMED",
        "authority_source_ids": ["fixture"],
        "detector_available": True,
        "detector_availability": "AVAILABLE",
        "runnable": True,
        "provider_deployment": None,
        "limitations": ["fixture only"],
    }
    manifest = complete_manifest()
    manifest["git_commit"] = "0" * 40
    manifest["authority_record"] = {"fixture": authority}
    schema = json.loads(
        (SCHEMA_ROOT / "experiment-manifest.schema.json").read_text(encoding="utf-8")
    )
    Draft202012Validator(schema, registry=registry()).validate(manifest)


def test_anthropic_provider_metadata_survives_canonical_schema_round_trip() -> None:
    authority = ANTHROPIC_AUTHORITY.to_dict()
    encoded = canonical_json_bytes(authority)
    decoded = json.loads(encoded)
    schema = json.loads((SCHEMA_ROOT / "authority-record.schema.json").read_text(encoding="utf-8"))
    Draft202012Validator(schema, registry=registry()).validate(decoded)
    assert canonical_json_bytes(decoded) == encoded
    assert decoded["provider_deployment"]["public_reference_relationship"] == (
        "RELATED_FAMILY_NOT_DEPLOYMENT_PARITY"
    )
    assert decoded["detector_availability"] == "ANNOUNCED_FORTHCOMING"
    validator = Draft202012Validator(schema, registry=registry())
    with pytest.raises(ValidationError):
        validator.validate({**decoded, "detector_available": True})
    with pytest.raises(ValidationError):
        validator.validate({**decoded, "runnable": True})


def test_localization_schema_rejects_ambiguous_coordinates() -> None:
    schema = json.loads((SCHEMA_ROOT / "example-record.schema.json").read_text(encoding="utf-8"))
    localization_schema = schema["$defs"]["localization"]
    validator = Draft202012Validator(
        {**localization_schema, "$defs": schema["$defs"]}, registry=registry()
    )
    value = {
        "coordinate_system": "CHARACTER",
        "marked_span_definition": "fixture",
        "marked_spans": [{"start": 0, "end": 2}],
        "predicted_span": {"start": 0, "end": 2},
        "overlap_token_count": 2,
        "union_token_count": 2,
        "iou": 1.0,
        "start_offset_error": 0,
        "end_offset_error": 0,
    }
    with pytest.raises(ValidationError):
        validator.validate(value)
