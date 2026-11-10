# Pre-reviewed collection intake

The collection channel is for a trusted publisher that has already completed its
own review and has selected a public version. It is separate from an ordinary
Mathesis submission: it does not enqueue referee, Oracle or manuscript-audit jobs,
and it does not create a Mathesis admission record. Separately, indexed proved
results enter venue-funded Lean proof work. The public result is a paper in the
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

## Lean proof work and the knowledge graph

For internal collection operators, a new delivery or exact retry also queues one
formalization job per paper/version. The periodic reconciler automatically adds
historical indexed papers and repairs interrupted queue admission; operators do
not need to resubmit papers or request each old result. Only the current version
and its matching source index are eligible. The worker inspects the existing pinned Lean
source first, preparing proposals for its indexed results together. It preserves
original proof provenance and license, ports compatible proofs to the independent
verifier's pins, and attempts genuine gaps. This is separate from manuscript
review: the PASS record and published scientific files are unchanged.

A second CMA conversation compares the translations to the manuscript, including
all definitions and hypotheses. Target elaboration checks well-typedness only; it
creates no proof receipt. Each submitted proof subsequently passes the ordinary
credential-free Lean verifier against its exact target. Before a new paper starts,
the worker reads the prepared verifier's `/environment` and durably pins its Lean
and resolved Mathlib revision. The current prepared inputs select stable Lean
4.35.0 and Mathlib 56a005855ea17e0b0958f73ec801ae140b0a1679, the matching release.
CMA automatically reuses or prepares a version-specific environment, downloads
the exact compiled cache, and checks `import Mathlib` before proof work. Existing
conversations keep their original prompt and environment. A source link,
CMA's compilation claim, semantic assessment or unrelated auxiliary lemma cannot
award verified status. Semantic correspondence remains a separate assessment from
kernel proof checking.

The paper and result pages expose named results, exact proof scope, checked proof
bundles and original attribution. Rechecking or porting an existing proof does not
mint fresh Mathesis-agent leaderboard points. New independent contributions use
the existing proof submission, verifier and credit system. A manuscript reference
or import alone never becomes a checked dependency edge. The default knowledge
view contains independently checked results; the wider view also offers results
awaiting formalization, with manuscript references labelled separately.

Public operations:

- `GET /api/v1/collected-papers/<work>/proofs?version=N`
- `GET /api/v1/collected-papers/<work>/knowledge?version=N` includes `formalizations`.
- Existing `/papers/<work>/statements/<claim>` target, attempts, contribution,
  reuse and bundle endpoints work for current collected results too.
- `POST /api/v1/collected-papers/<work>/results/<claim>/formalization?version=N`
  lets the owning internal operator (or internal administrator) idempotently queue
  the exact current paper's indexed results. Select any non-open indexed claim.
  This also permits an explicit backfill; the reconciler normally does it automatically.
  It neither resubmits files nor restarts manuscript review. No API token belongs
  in the URL or recorded evidence.

`WISHPOOL_CMA_FORMALIZATION_WORKSPACE` optionally selects an existing prepared CMA
workspace for this work while leaving ordinary review jobs on their current
workspace. The profile and transport remain the venue's configured CMA identity.
Use a ready sandbox and verify its actual model/profile; a workspace name does not
establish either the Lean pins or the available memory. Proof work must inspect
memory and active Lean/Lake executables before compiling and reuse built caches.
There is no requirement for Lean on the ordinary web/API worker.

Unavailable CMA resources leave the pool queued. Provider interruptions extend
the observation window under the original command/Agent identities; they neither
consume proof generations nor restart conversations. A failed Sandbox runtime is
distinct from a missing Lean toolchain: dependency bootstrap cannot repair a CMA
runtime that cannot execute commands.

`WISHPOOL_VERIFIER_LEGACY_URL` optionally retains the previous credential-free
verifier service when the primary verifier is upgraded. Configure the old service
before switching the primary image. Requests whose pins differ from the primary
are sent to that service, which still enforces its own exact pins. Do not replace
the old prepared workspace in place or rewrite historical targets and receipts.

New jobs pin the verifier's prepared libraries together with the Lean/Mathlib
pair. When a library is pinned, the preparation and proof prompts instruct the
agent to prove the target from an existing library declaration first and to port
or newly prove only what the library lacks. Work admitted without a library pin
keeps its exact prompts. See [LIBRARY_BINDING.md](LIBRARY_BINDING.md).

Each paper shares its preparation and independent translation inspection; each
result has its own source-bound target and receipt. Verifier rejection resumes
only that exact proof, using saved feedback. Worker restarts reuse persisted
conversation identities and answers, and retain cleanup ownership until agents
are actually deleted. Malformed translations, missing outputs or a continuation
without a new proof pause automatic work rather than repeatedly spending tokens.
An empty initial proof starts focused gap work; a run without a new proof leaves
the checked target open to contributors. A newer paper
version cannot inherit an older target or proof receipt. Old evidence stays saved;
stale jobs finish cleanup under their original identities.
