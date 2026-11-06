# Pre-reviewed collection intake

The collection channel is for a trusted publisher that has already completed its
own review and has selected a public version. It is separate from an ordinary
Mathesis submission: it does not enqueue referee, Oracle, audit or Lean jobs, and it
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
bibliographic: title, authors, abstract, bilingual reading guide, PDF/source
links and optional pinned Lean source.

This path is intentionally explicit. It is not a background import of every
upstream result, and ordinary external submissions continue through
`/manuscript-deliveries` and the independent Mathesis review pipeline. A real delivery is collected only when the producer calls this endpoint; a
source repository or an archive DOI alone does not create a Mathesis paper.

## Independent reading guide

A successful collection saves its reviewed scientific files immediately and queues
one durable, venue-funded CMA explanation for that exact work/version. This is
exposition, not another review or publication decision. The task reads the pinned
source ZIP and public manuscript metadata; private producer review evidence is
never part of the prompt or public response. It explains the question, definitions,
main result, proof mechanism, prior attribution and exact remaining scope in five
to eight English and Chinese paragraphs (normally 450–800 English words and
900–1600 Chinese characters). It cannot award verification or formalization credit.

A generated explanation lives in a separately revision-fenced sidecar. The
original request digest, PASS evidence, reviewed source/PDF bytes, authors,
publication date and retry receipt stay immutable. Until it is ready, any supplied
upstream explanation remains available. Existing public versions can be queued by
an administrator with `POST /api/v1/collected-papers/<work-id>/plain-language/request?version=N`.
Exact delivery retries repair queue admission without creating a manuscript or
restarting the referee. The same durable CMA run and job are retained through
worker restarts and asynchronous agent deletion.

## Published archive DOI

Mathesis consumes an existing published Zenodo DOI; collection does not register a
new DOI or represent Zenodo archiving as a Mathesis publication decision. Producers
may supply `paper.archive_doi` with the **exact version** identifier, for example
`10.5281/zenodo.23957560`. Omit it until the archive has actually published the
record. Reserved identifiers, drafts and concept identifiers in the exact-version
slot are rejected. The server reads the public production Zenodo record and checks
that both its PDF and source ZIP have the same sizes and SHA-256 values as the
reviewed files. Ambiguous matching files or unavailable evidence cannot add a DOI.
The registry's concept DOI is retained separately; the reader and citation use the
exact-version DOI. There is no dependence on private archive credentials.

For an already collected version, the owning trusted operator or an administrator
can attach a DOI without resubmitting the scientific delivery:

```http
POST /api/v1/collected-papers/<work-id>/archive?version=N
Content-Type: application/json
Authorization: Bearer <NyxID token>

{
  "doi": "10.5281/zenodo.23957560",
  "pdf_sha256": "<exact reviewed PDF SHA-256>",
  "source_sha256": "<exact reviewed source ZIP SHA-256>"
}
```

This set-once, file-bound sidecar preserves the original delivery JSON, request
digest, files, publication time, review evidence and receipt. It creates no review,
exposition or Lean job. Identical retries return the same public paper; a different
DOI conflicts. Each version has its own archive citation; a newer manuscript never
inherits an earlier version's DOI. Adding a DOI to an old delivery envelope would
change its digest and is intentionally refused: use this enrichment endpoint.

Public collection responses optionally include
`archive: {doi, concept_doi?}`; they omit verification internals and unverified
input metadata. Existing clients and deliveries without DOI remain compatible.
The archive, paper sidebar, plain-text citation and reference-manager metadata
link the exact DOI. Archive search also accepts the DOI.
