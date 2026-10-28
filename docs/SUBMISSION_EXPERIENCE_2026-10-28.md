# Submission and review experience — 28 October 2026

The My work list previously requested the entire private referee dossier per row.
A production request measured 12.15 seconds and returned several megabytes; a
missing loaded dossier was rendered as “Review progress unavailable” regardless
of loading, network failure or review state. The underlying current submissions
were review_complete with delivered current-round letters. This does not establish
the exact cause of every browser request failure. The list now requests only a
small authorized current-version summary and represents loading, request failure,
interruption and delivered reports separately.

The submission form has four horizontal tabs, structured author names and
affiliations, and PDF/TeX/archive/multiple-file intake. Revisions support the same
formats. PDF intake uses Poppler with bounded text extraction and preserves the
original PDF. The author checks the PDF and may inspect extracted results before
starting review. Selectable text and numbered statement headings are required;
scans and unreadable statement boundaries return actionable errors.

Authors see one delivered final report and one download, with earlier reports
in a collapsed history. Provider steps and internal recommendations are retained
in the separate staff route. No unfinished draft is presented as a final report.
New report prompts are pinned with letter_contract 1, while legacy admitted
prompts remain byte-identical. Applied publication decisions remain authoritative:
acceptance/refusal for publication, no verdict for private reviews. Historical
reports, decisions and review-only privacy are not rewritten.

Archive search accepts titles, authors, abstracts, DOI and record identifiers.
There is no separate Authors discovery page; existing author profiles remain
linked to their work. Community formalization starts by choosing an existing
conjecture/theorem and preserves both original-author and formalizer credit.

Validation includes frontend interaction/polling/privacy tests, actual PDF
reading and byte preservation, malformed/no-text PDF rejection, summary access
and current-round fencing, legacy prompt stability, automatic verdict mapping,
and existing workspace tests. Normal CI gates remain unchanged. Deployment
uses isolated build jobs for the web and API/worker images, preserving the
existing runtime and configuration. The browser connection returned
“unsupported Codex auth method: apikey”, so screenshot-based visual verification
was unavailable in this session; DOM interaction tests are not a substitute for
that visual check.
