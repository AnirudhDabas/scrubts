from waterlarp.schema_validation import _registry


def test_schema_registry_is_complete_and_local() -> None:
    registry, schemas = _registry()
    assert registry
    assert set(schemas) == {
        "aggregate.schema.json",
        "authority-record.schema.json",
        "example-record.schema.json",
        "experiment-manifest.schema.json",
    }
    assert all(schema["$id"].startswith("https://scrubts.dev/") for schema in schemas.values())
