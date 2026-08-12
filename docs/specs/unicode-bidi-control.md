# Unicode Bidi_Control reporting

## Scope

This contract governs a future deterministic scanner that reports occurrences
of the Unicode property `Bidi_Control` in a byte artifact interpreted as UTF-8.
It defines source authority and reporting semantics only; it does not establish
production support.

The mechanism identifier is `unicode.bidi_control`. The mechanism/data version
is `17.0.0`.

## Source authority

Property and data semantics are governed by Unicode Standard Annex #44,
Unicode Character Database. Membership is governed only by the explicit
`Bidi_Control` records in Unicode 17.0.0 `PropList.txt` at
`research/library/unicode/17.0.0/PropList.txt`:

- byte size: 145,465;
- SHA-256: `130dcddcaadaf071008bdfce1e7743e04fdfbc910886f017d9f9ac931d8c64dd`;
- property extent: 4 explicit records covering 12 code points.

The data is distributed under Unicode License V3. The local license text is
`research/library/unicode/17.0.0/UNICODE-LICENSE.txt`, with SHA-256
`e7a93b009565cfce55919a381437ac4db883e9da2126fa28b91d12732bc53d96`.

Unicode Standard Annex #9, Unicode Bidirectional Algorithm, Revision 51 for
Unicode 17.0.0, governs the directional-formatting character names,
abbreviations, and behavior. Unicode Technical Standard #55, Unicode Source
Code Handling, supplies security and display interpretation. Neither source
changes membership from the pinned `PropList.txt` records.

## Semantic source binding

The ordered `Bidi_Control` records have a compact semantic binding independent
of comments and whitespace in the raw UCD file. Serialize one record per range
as an uppercase six-hex-digit start, literal `..`, uppercase six-hex-digit end,
and LF terminator. Serialize a singleton as `START..START`.

```text
00061C..00061C
00200E..00200F
00202A..00202E
002066..002069
```

The canonical serialization is 60 bytes and has SHA-256
`217873f8bf2ca674f32afff23b3dc5fd81e4b55b5f6aa978c63417ad29f22674`.

## Canonical control identities

Reports use the canonical code point and UAX #9 abbreviation; they never emit
the raw invisible control as terminal evidence.

| Code point | UAX #9 abbreviation | Name |
|---|---|---|
| `U+061C` | `ALM` | ARABIC LETTER MARK |
| `U+200E` | `LRM` | LEFT-TO-RIGHT MARK |
| `U+200F` | `RLM` | RIGHT-TO-LEFT MARK |
| `U+202A` | `LRE` | LEFT-TO-RIGHT EMBEDDING |
| `U+202B` | `RLE` | RIGHT-TO-LEFT EMBEDDING |
| `U+202C` | `PDF` | POP DIRECTIONAL FORMATTING |
| `U+202D` | `LRO` | LEFT-TO-RIGHT OVERRIDE |
| `U+202E` | `RLO` | RIGHT-TO-LEFT OVERRIDE |
| `U+2066` | `LRI` | LEFT-TO-RIGHT ISOLATE |
| `U+2067` | `RLI` | RIGHT-TO-LEFT ISOLATE |
| `U+2068` | `FSI` | FIRST STRONG ISOLATE |
| `U+2069` | `PDI` | POP DIRECTIONAL ISOLATE |

## Status semantics

The future scanner validates the complete input as UTF-8 before classifying
property membership:

- `PRESENT`: valid UTF-8 containing one or more Unicode 17.0.0 `Bidi_Control`
  scalar values;
- `ABSENT`: valid UTF-8 containing no Unicode 17.0.0 `Bidi_Control` scalar
  values;
- `INVALID`: malformed or incomplete UTF-8.

No normal `UNKNOWN` state is expected for this deterministic property scanner.
Malformed or incomplete UTF-8 is never `ABSENT`. The final finding is `INVALID`
and discards all property observations from any valid prefix, including counts
and locations.

## Locations and bounded evidence

For valid UTF-8, scan the complete artifact and count every occurrence,
including repeated values. Retain at most the first 256 locations in input
order. Report `locations_truncated` if and only if the total occurrence count
exceeds 256.

Valid findings use the existing evidence names `total_occurrence_count`,
`locations_truncated`, and `locations`. Each retained location records:

- canonical `code_point`, such as `U+202E`;
- canonical UAX #9 `abbreviation`, such as `RLO`;
- zero-based `byte_offset`, identifying the first byte of the scalar's original
  UTF-8 encoding;
- zero-based `scalar_offset`, counting decoded Unicode scalar values before the
  occurrence.

The `locations` value is a compact JSON array of objects in input order. This
uses the existing string-valued evidence model and does not change report schema
0.1. An `INVALID` finding may retain complete-artifact UTF-8 validation evidence
and an explanatory limitation, but no prefix occurrence, count, or location
evidence.

## Independent property findings

A Unicode scalar may satisfy multiple properties. Every Unicode 17.0.0
`Bidi_Control` scalar is also covered by the existing
`unicode.default_ignorable_code_point` membership data. One occurrence may
therefore cause both mechanisms to report `PRESENT`. Findings remain independent
and are not deduplicated or collapsed.

## Reporting invariants and non-claims

`Bidi_Control` presence is a neutral Unicode-property observation. Unicode
recognizes legitimate uses of directional-formatting characters. Presence does
not mean or imply:

- Trojan Source detection;
- malicious content;
- spoofing detection;
- a source-code vulnerability;
- AI-generated content;
- watermark content;
- Claude content;
- unsafe content.

The mechanism does not perform contextual source-code security analysis, UAX #9
reordering, normalization, confusable detection, sanitization or removal,
file-type detection, C2PA inspection, or content transformation.

## Implementation boundary

This contract does not choose a streaming-decoder architecture. Milestone 3C
must review whether the existing DICP scanner and Bidi_Control support should
consume one shared validated scalar stream. Any refactor must be justified by
that implementation, remain small, and must not introduce a generic scanner,
registry, plugin, or Unicode framework.
