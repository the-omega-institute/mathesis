# Preserve reports containing unproved conjectures

A private graph manuscript's referee returned valid JSON with 28 claim readings.
The two open conjectures used `shape: null`. Deserializing the optional proof
shape into a string rejected the entire structured report, including the other
readings, recommendation and knowledge-query proposals. The raw text remained
available to the source auditor, but the structured projection was lost.

The referee adapter now treats a null proof shape like an absent shape: the
empty string. Domain mapping still requires a recognized shape and the matching
witness rules for every proved claim. It still excludes open conjectures from
proved-result readings. No shape, proof or positive assessment is invented.
Non-string, non-null values remain invalid. Raw answers, prompts, admitted task
identities and settled historical reports are unchanged.

Regression checks cover a mixed theorem/conjecture report, preserved retrieval
proposals, a proved claim with a missing shape that must remain unusable, and
invalid value types. This is a downstream response-decoding repair; it changes
no upstream provider or publication rule.
