# Style

This project should read like small, deliberate systems/research software.

## Code

- Prefer explicit ordinary functions/data types over framework-shaped abstractions.
- No `Manager`, `Engine`, `Registry`, `Factory`, or plugin trait until concrete repeated behavior earns it.
- Avoid catch-all `utils` modules. Put behavior beside the domain type that owns it.
- User-controlled input must not reach panic/unwrap paths.
- Comments explain non-obvious invariants, external-spec decisions, or numerical assumptions; they do not narrate syntax.
- Stable machine output is a contract. Human rendering may evolve separately.
- Add dependencies only when they reduce more risk/complexity than they add.

## Documentation

- Prefer primary-source links.
- State limitations next to capabilities.
- Separate vendor-reported, replicated, measured, inferred, and unknown claims.
- Avoid marketing filler and “state of the art” unless tied to a specific cited benchmark/definition.
- Diagrams should explain a mechanism or data flow, not decorate the README.

## Research

- Raw/sample-level results are the source of truth; aggregate tables/plots are generated.
- Record exact code/model/tokenizer/detector revisions and environment.
- Never hand-edit a published number.
- A semantic evaluator must be validated against a frozen human-labeled set before becoming a reported metric.
