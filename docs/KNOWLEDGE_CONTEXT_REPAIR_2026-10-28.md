# Readable source evidence within the prompt budget

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
