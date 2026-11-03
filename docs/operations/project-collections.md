# Versioned research collections

Mathesis can display explicitly selected project publications using their existing
upstream manuscript review. This collection is separate from independent Mathesis
admission: collection entries do not receive a Mathesis acceptance, verification
badge, public review report, or accepted-only generated explanation.

The first collection contains ten Trureturing short notes. Public pages retain
the authors in their original order, original questions and source citations,
the exact reviewed PDF and source ZIP, and Lean links pinned to a source commit.
Original question identifiers (including arXiv IDs) are sources, not identifiers
assigned to the new note. Several questions may share one note. Associated problem
counts describe that relationship; they do not assert that every related question
has been solved. In particular, the A391810 note does not settle its original
first-zero clause. A linked Lean file is evidence to inspect, not a claim of a
completed independent Mathesis audit.

## Updating the collection

Use the existing coordinator selections and reviewed published artifacts. No new
upstream schema or release workflow is required. The adapter accepts
`short-note.v1` or the existing `outreach-mathesis-handoff.v1` scientific manifest.
Stage the PDF, scientific manifest and matching source ZIP under
`<artifacts>/<work-id>/`. Keep coordinator selections and checked confirmations
outside the repository. Private upstream artifact locations are never linked in
the public snapshot.

An operator must authorize public collection inclusion. Run from the repository:

```sh
python3 docs/operations/import_project_notes.py \
  --selections /private/path/to/intake \
  --artifacts /private/path/to/scientific-artifacts \
  --published-on YYYY-MM-DD \
  --authorize-publication
python3 -m unittest discover -s docs/operations -v
```

The importer checks upstream PASS/result binding, work identity and revision,
authors/title, public publication intent, CC BY 4.0, scientific file hashes and
sizes, matching source confirmation, and pinned public Lean evidence. It validates
the complete batch before writing. The operator remains responsible for inspecting
the selected scientific files and confirming publication rights; this script does
not authenticate a coordinator or automatically trust a new external producer.

It copies only the scientific PDF and source ZIP into
`web/public/collections/trureturing/<work-id>/v<revision>/` and writes a whitelist of
scientific metadata to `web/src/data/project-notes.json`. Private review reasons,
coordinator/task/account identities and delivery receipts are not copied.
Published versions and their original inclusion dates are immutable. A new
upstream revision adds a version, and the stable work URL selects the latest;
`?version=N` continues to select the exact historical version. A missing requested
version fails explicitly.

Review the resulting diff, run the frontend checks, and use the existing image
publication and deployment process. This is an explicit, reviewable batch import,
not a background sync of all upstream problems. Private Mathesis submissions and
their independent decisions continue through the existing review workflow.

Global archive search includes these notes by title, author, abstract, problem and
source identifier. It does not infer MSC categories. Home and unfiltered archive
pages show three notes with a link to the complete collection. All public assets
are checked by SHA-256 in the frontend tests and must be checked after deployment.

The shared page width is 82rem including responsive gutters. Home, navigation,
content and footer use that same centered width; mobile gutters stay at 1.25rem.
