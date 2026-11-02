# Interface language and accepted-paper explanations

The header owns one English / 中文 preference. English is the default; `mathesis.language` in browser storage persists the choice and synchronizes other tabs. Navigation, submission controls, dates and the plain-language explanation follow this setting. Changing language re-renders the current route without changing its key, submission step or controlled form values.

Interface copy is explicitly translated in `web/src/i18n`. Manuscript titles, abstracts, mathematical statements, author names, references, contributed code and the original review text keep their source language. Both explanations are prepared from the same exact manuscript version; neither is a translation of a private report.

Only acceptance queues a plain-language explanation. A replay of an accepted decision recovers a queue write that failed after acceptance was saved. The worker, active execution fence and write endpoints require an accepted current manuscript. Existing explanation text on an unaccepted manuscript is retained privately and is hidden by the author interface; it is reused if the identical version is later accepted. No explanation job starts during draft confirmation.

An accepted explanation remains bound to the source hash, manuscript version, statement revision and generation. Public projection uses the accepted snapshot and its visibility policy. A later private revision cannot replace the public predecessor's explanation. Generating or retrying an explanation does not repeat the mathematical review, alter its decision, or confer Lean verification.

The header search has a single input boundary with an internal submit icon. The visible placeholder is translated, the label remains available to assistive technology, and Enter and button submission use the same archive query route.
