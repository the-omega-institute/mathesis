# Published manuscript intake

Mathesis can collect explicitly selected published manuscripts using their existing
manuscript review. They appear as ordinary papers in the shared research archive
and homepage, without producer branding, project navigation, collection sections
or workflow provenance in the public page or citation. Internal intake evidence
remains separate from scientific metadata. Inclusion does not invent an independent
Mathesis acceptance, verification badge, public review report.

Collection inclusion does not submit a manuscript for independent Mathesis review
or enqueue Oracle/referee/audit work. An independent venue-funded CMA task may
produce the bilingual reading guide from the exact public manuscript source. Upstream-reviewed published notes are collected after
checking their published files, provenance, rights and version identity. Do not
re-review the full project as a prerequisite for showing its publications.
Independent Mathesis review is a separate, explicitly requested submission.
Previously saved private review attempts are retained; importing or updating a
collection neither restarts those attempts nor changes their decisions.

The first collection contains ten Trureturing short notes. Public pages retain
the authors in their original order, original questions and source citations,
the exact reviewed PDF and source ZIP, and Lean links pinned to a source commit.
Original question identifiers (including arXiv IDs) are sources, not identifiers
assigned to the new note. Several questions may share one note. Associated problem
counts describe that relationship; they do not assert that every related question
has been solved. In particular, the A391810 note does not settle its original
first-zero clause. A linked Lean file is evidence to inspect, not a claim of a
completed independent Mathesis audit.

## Updating the published manuscripts

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
source identifier. It does not infer MSC categories. The archive lists all selected
manuscripts alongside API papers in one dated bibliography. Local publications
appear on the first page only; following an API cursor does not repeat them. The
homepage shows the three most recent publications across both intake paths. All
public assets are checked by SHA-256 in the frontend tests and after deployment.

Archive lists use compact bibliographic entries: title, author names, date, version
and reading/source links. Abstracts, original mathematical source citations and
formalization details remain on the paper page. New links and version citations
use `/papers/<work-id>?version=N`. Previously shared `/projects/trureturing` and
`/projects/trureturing/notes/<work-id>?version=N` routes open the same archive or
paper; no separate project page remains. Previously published asset URLs stay
available so existing PDF and source references continue to resolve.

The shared page width is 89rem including responsive gutters. Home, navigation,
content and footer use that same centered width; mobile gutters stay at 1.25rem.

## Automatic pre-reviewed intake

New upstream short notes should use the trusted collection endpoint described in
[`docs/PRE_REVIEWED_COLLECTION.md`](../PRE_REVIEWED_COLLECTION.md). The static
importer remains for historical backfills and artifact recovery; it is not the
normal path for a new published note. A successful collection receipt is already
public and does not start an independent Mathesis review. Do not copy its private
review record into `project-notes.json` or into the reader page.
