# Unicode Default_Ignorable_Code_Point reporting

## Scope

This contract governs a deterministic scanner that reports occurrences of the
Unicode property `Default_Ignorable_Code_Point` (DICP) in a byte artifact
interpreted as UTF-8. It is a neutral Unicode-property observation. Some DICP
values have legitimate language, shaping, emoji, and formatting uses.

## Source authority

Property semantics are governed by Unicode Standard Annex #44, Unicode
Character Database. Membership is governed only by the explicit
`Default_Ignorable_Code_Point` entries in Unicode 17.0.0
`DerivedCoreProperties.txt` at
`research/library/unicode/17.0.0/DerivedCoreProperties.txt`:

- byte size: 1,134,783;
- SHA-256: `24c7fed1195c482faaefd5c1e7eb821c5ee1fb6de07ecdbaa64b56a99da22c08`;
- property extent: 27 explicit ranges covering 4,174 code points.

The data is distributed under Unicode License V3. The local license text is
`research/library/unicode/17.0.0/UNICODE-LICENSE.txt`, with SHA-256
`e7a93b009565cfce55919a381437ac4db883e9da2126fa28b91d12732bc53d96`.

Unicode Technical Standard #55, Unicode Source Code Handling, supplies
interpretation and security guidance. Unicode Technical Standard #39, Unicode
Security Mechanisms, is reserved for future confusable analysis and does not
govern DICP membership in this milestone.

## Status semantics

The scanner validates the complete input as UTF-8 before classifying property
membership:

- `PRESENT`: valid UTF-8 containing one or more Unicode 17.0.0 DICP values;
- `ABSENT`: valid UTF-8 containing no Unicode 17.0.0 DICP values;
- `INVALID`: malformed or incomplete UTF-8.

Malformed or incomplete UTF-8 is never `ABSENT`. The scanner does not classify
a partially decoded prefix as `PRESENT` or `ABSENT` when the complete input is
invalid.

## Locations and bounded evidence

Each retained occurrence has both:

- a zero-based byte offset: the index of the first byte of that scalar value's
  UTF-8 encoding, counted from the start of the artifact;
- a zero-based Unicode scalar-value offset: the number of decoded Unicode
  scalar values preceding the occurrence. This is not a byte, UTF-16 code-unit,
  grapheme-cluster, or display-column offset.

Every occurrence is counted, including repeated values. Locations are retained
in increasing input order, with at most the first 256 retained. Evidence reports
the total occurrence count and whether locations were truncated; truncation is
reported if and only if the total count exceeds 256. For the same input,
configuration, scanner version, and Unicode data version, status, counts,
retained locations, truncation, and report ordering are identical.

## Reporting invariants

Reports describe only whether the pinned Unicode property occurs. Wording must
not characterize presence or absence as malicious, suspicious, AI-generated,
watermarked, tracking metadata, unsafe, or removable. The scanner reads the
artifact without changing its bytes and does not emit transformed content.

## Non-goals

This scanner does not perform bidi-control security analysis, normalization,
confusable analysis, sanitization or removal, C2PA inspection, statistical
watermark detection, Claude-specific detection, or WaterLARP experiments. It
makes no claim about any of those mechanisms from DICP presence or absence.
