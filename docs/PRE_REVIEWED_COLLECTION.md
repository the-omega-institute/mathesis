# Pre-reviewed collection intake

The collection channel is for a trusted publisher that has already completed its
own review and has selected a public version. It is separate from an ordinary
Mathesis submission: it does not enqueue referee, Oracle, CMA or Lean jobs, and it
does not create a Mathesis admission record. The public result is a paper in the
same archive, with its authors, abstract, sources, exact PDF and source archive.

The endpoint is `POST /api/v1/pre-reviewed-deliveries` as an authenticated
multipart request with three fields:

- `metadata`: a `mathesis.pre-reviewed-delivery.v1` JSON envelope;
- `pdf`: the exact reviewed PDF;
- `source`: the exact reviewed source ZIP.

The operator identity must be listed in `WISHPOOL_INTERNAL_SUBJECTS`. The envelope
requires a producer/work slug, a monotonically versioned paper, ordered authors,
CC-BY-4.0, exact lowercase SHA-256 values for both files, an explicit `PASS` review
record, and any pinned public Lean source. A retry of the same bytes returns the
same receipt. A changed payload under a published version is rejected. A new
version must name the exact current predecessor; published versions are immutable.

Public readers use `GET /api/v1/collected-papers` and
`GET /api/v1/collected-papers/<work-id>?version=N`. File URLs require an exact
version. The reader projection omits the producer, review record, task IDs, hashes
and internal account identity; hashes remain available for file integrity and are
stored in the private collection record. The web archive keeps the reader view
bibliographic: title, authors, abstract, short bilingual explanation, PDF/source
links and optional pinned Lean source.

This path is intentionally explicit. It is not a background import of every
upstream result, and ordinary external submissions continue through
`/manuscript-deliveries` and the independent Mathesis review pipeline. The first
real upstream delivery is still pending until the producer calls this endpoint.
