# Do not accept an unrelated object as a structured referee reading

Real broker replies exposed two cases with no usable structured referee output:
an explicit attachment-access failure, and substantive raw reports whose outer
JSON contained unescaped quotation marks. For unfenced replies the generic
scanner can locate a valid inner object after malformed outer JSON. Serde
defaults then turn unrelated fields into an empty `RefereeOut` without an error.

Paper and paper-problem referee processing now require at least a nonempty
summary, a claim reading or a concern. Otherwise they use the existing fallback:
preserve the entire original answer and record an explicit parsing limitation.
Partial readings still reach the existing domain sanitization. A recommendation
alone is not a mathematical reading. No ambiguous quotation marks are repaired,
no assessment is invented and no publication rule changes.

This guard makes the limitation explicit; it does not restore inaccessible
attachments or claim to recover malformed structured evidence. Independent
source auditing can continue using the retained manuscript and raw answer.
Stored settled results, prompt bytes and admitted request identities are
unchanged. Operator exports must distinguish delivered letters with such
limitations from reviews whose structured inputs were available throughout.
