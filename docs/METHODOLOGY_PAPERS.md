# Methodology papers

Mathesis keeps methodology and meta-science work in a separate `methodology`
track. The Papers directory opens on Mathematics; its Methodology tab uses
`kind=methodology`. MSC codes are optional. Mathematical browsing uses
`exclude_kind=methodology`; API requests without filters retain all kinds.

External authors submit `kind: "methodology"` through the ordinary submission
API. The same correctness, supported literature, utility and content gates apply.
Routine assembly, renaming, prompt wrapping and recombination are `bind_only`.
A content reading needs a live non-binding mechanism, invariant, admission
protocol, architectural contract or controlled causal evaluation, with concrete
manuscript witnesses. `utility.kind: "methodology"` requires supported status,
a rationale, the methodological problem solved and the advance demonstrated.
A precisely audited settlement of a named, sourced open methodological problem or
benchmark retains the separate settlement route. Missing evidence is
`undetermined`. Existing mathematics prompt contracts remain unchanged.

Trusted internal operators instead use the existing
[pre-reviewed collection intake](PRE_REVIEWED_COLLECTION.md). `INTERNAL_EDITORIAL`
records an operator's editorial selection; it bypasses automated referee, Oracle
and source-audit review, as does `PASS`. Both require exact file hashes, a nonempty
private review record and membership in `WISHPOOL_INTERNAL_SUBJECTS`. Collection
creates no ordinary Mathesis admission record. Existing explanation/indexing
sidecars are independent of review and retain their configured behavior.

## Repository delivery

Use `sdk/collection/deliver_collected_paper.py` (Python 3.11+, standard library).
Stage only manuscript source inputs in `manuscript/`, put the compiled PDF at
`paper.pdf`, and commit `mathesis.json` with the following delivery envelope:

```json
{
  "schema": "mathesis.pre-reviewed-delivery.v1",
  "producer": "mathesis-papers",
  "work_id": "human-in-the-loop-automated-research",
  "version": 1,
  "previous_version": null,
  "authorize_publication": true,
  "paper": {
    "kind": "methodology",
    "title": "Human-in-the-loop automated research",
    "abstract": "Replace with the actual manuscript abstract.",
    "authors": [{"name": "Replace with the actual author name"}],
    "license": "CC-BY-4.0",
    "msc": [],
    "repository_url": "https://github.com/mathesis-papers/human-in-the-loop-automated-research",
    "version_label": "v1.0",
    "changelog": "Initial editorially selected version."
  },
  "review": {
    "verdict": "INTERNAL_EDITORIAL",
    "record": {"editorial": "Replace with the actual editorial selection evidence."}
  }
}
```

The helper fills both SHA-256 fields. Source ZIPs use sorted relative paths,
fixed timestamps and modes, and stored (uncompressed) entries for stable bytes
across platforms. File modification times and permissions do not affect hashes.
Symlinks, private/generated directories, empty archives, excessive file counts
and uploads above the server's 30 MiB file limit are refused. The helper performs
local envelope validation; server validation and operator authorization remain
final. It does not compile LaTeX or run any model.

```sh
python3 /path/to/mathesis/sdk/collection/deliver_collected_paper.py /path/to/paper --dry-run
MATHESIS_URL=https://mathesis.example MATHESIS_TOKEN="$TOKEN" \
  python3 /path/to/mathesis/sdk/collection/deliver_collected_paper.py /path/to/paper \
  --auto-previous-version
```

`--metadata`, `--pdf` and `--source-dir` select paths relative to the paper repo.
Optional overrides are `--kind`, `--repository-url`, `--issues-url`,
`--version-label`, `--changelog` and `--verdict`. `MATHESIS_URL` accepts the site
root or a root ending in `/api/v1`. HTTPS is required except on loopback for
local development. Redirects are refused. Dry runs print the envelope, including
the private review record, and perform no network request unless predecessor
lookup was explicitly requested. Keep that output private.

For a revision, advance the positive integer `version`, supply the exact current
`previous_version`, and update `version_label` and `changelog`. Versions need not
be consecutive. `--auto-previous-version` reads the current public version; a
retry of that same version preserves its original predecessor. Concurrent
successors still conflict at the server. Exact retries require the same envelope,
PDF/source bytes and upload filenames. This helper always uses `paper.pdf` and
`source.zip` as upload filenames, independently of input paths. Preserve the helper
revision and inputs for reproducibility; do not use it to retry an older delivery
that used different filenames. Changed metadata under an existing version conflicts.

Public detail responses include the complete predecessor history in `versions`,
oldest first, with `revision`, date, files, optional archive DOI, `version_label`
and `changelog`. Reading an old version still shows the complete current history.
Each link pins its version. Public metadata includes the effective kind and
optional repository/release fields; review records remain private. If no issues
URL is supplied, a repository exposes its `/issues` URL. URLs must be literal
`https://github.com/<owner>/<repo>` and the matching `/issues` path, with ASCII
slugs, no credentials, query, fragment, encoding or trailing slash. Conjectures
use their dedicated intake and are refused by this channel.

The catalog preserves methodology works, repository metadata and available version
history. `methodology_papers` is separate from `public_papers`; methodology works
and their nodes do not contribute to mathematical/MSC statistics. Catalog generation
and repository materials never confer scientific or Lean verification.

Copy [the tag-push workflow template](templates/deliver-methodology-paper.yml)
into a paper repository after configuring its URL, trusted operator secret and
pinned Mathesis helper revision. It is a template only; nothing in this change
installs a workflow in another repository or changes a running service.
