# Readable review evidence and completed mathematical reports

A production review with 32 statements had 64 saved source-query responses. Its
first response was 276,144 bytes, with 100 declaration records across six source
modules. The model-facing budget was 12,484 bytes per receipt. Shortening source
text still left 101,713 bytes, principally declaration metadata, so the old
projection discarded the entire result. The completed query and its saved bytes
were intact, but the reviewer correctly reported that it received no source text.

New rounds pin knowledge contract 4. Projection now measures the complete receipt
entry and retains bounded source prefixes while reducing excessive declaration
catalogues. It keeps candidates from each ranked module when possible, then
shortens excerpts and reduces returned hits if necessary. The original source
hashes, declaration identities, response checksum and stored query remain exact.
Each omission sets the existing truncation and prompt-limitation fields. A result
that cannot fit remains unavailable; omitted hits never become an empty search.
This does not establish entailment, novelty, complete literature coverage or a
formal certificate. The existing publication and visible-premise checks apply.

Contracts 0–3 retain their existing projection. Active tasks keep their input
digests and provider identities; no historical report or decision is rewritten.
This repair changes only downstream consumption in Mathesis. Source publication,
ranking, upstream APIs, shared CI and deployment configuration are unchanged.

Regression checks cover the measured 64-receipt budget with a dense 100-declaration
catalogue, UTF-8 source prefixes, exact saved-output preservation, deterministic
replay, small results, missing responses, explicit omissions and byte limits.
The existing workflow checks durable source delivery to Oracle and the audit,
historical contract replay, publication gates, and private review behavior.

## Literal LaTeX backslashes in completed CMA output

A second completed source audit returned 86,090 characters of otherwise valid
JSON with one unescaped LaTeX `\delta` inside a string. The earlier delimiter-only
normalization rejected the entire result. The same bounded, raw-preserving
normalizer now encodes any backslash inside a string whose next character is not
one of the JSON escape codes. This preserves literal text; it does not interpret
LaTeX or repair object structure. Existing valid escapes, including Unicode, stay
exact. Malformed/truncated Unicode escapes, raw control characters, nonobjects,
prose, missing delimiters and trailing content remain rejected. The original
provider output is saved separately from the normalized JSON, and replay needs
no additional provider request. Already settled failures remain historical.

Regression coverage includes the actual mixed `\\Delta=\delta=3` pattern, durable
raw-output preservation, unchanged valid escapes, Unicode and structural errors.

## Apply the attachment limit to attached review instructions

A three-statement Selective LOCC manuscript failed before Oracle admission because
its instructions and saved source evidence exceeded the obsolete 500,000-character
inline-prompt check. Long requests already travel as a text attachment beside the
PDF. Validation now measures that attachment in UTF-8 bytes, capped at 2,000,000.
This keeps the old worst-case byte bound (500,000 four-byte characters), so the
largest allowed PDF and instructions still fit the broker's 16 MiB request body.
The inline composer receives the same short directive; full evidence, client
references and task request bytes stay unchanged for previously admitted work.

HTTP and CLI regressions send more than 500,000 characters as an exact attachment,
including the final mathematical statement and the separate PDF. Byte, base64 and
combined-body limits remain tested. No source evidence is silently truncated.
