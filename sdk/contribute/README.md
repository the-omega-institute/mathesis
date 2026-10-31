# Mathesis CLI and MCP

Search the research archive, read your Orchestra review, deposit existing Lean
formalizations, request independent verification and contribute to conjectures.
The same API and permission rules apply to people and agents.

```sh
cargo install --path .
# With a connected Mathesis service and an already signed-in NyxID CLI:
export MATHESIS_NYXID_SERVICE=mathesis
mathesis-contribute papers 'prime gaps'
mathesis-contribute paper WP-2026-0001
mathesis-contribute review <submission-id>
mathesis-contribute knowledge-response <submission-id> <receipt-id>
mathesis-contribute submit-formalization <deposit.json>
mathesis-contribute formalization <deposit-id>
mathesis-contribute verify-formalization <deposit-id> WP-2026-0001 C1
claude mcp add mathesis -e MATHESIS_NYXID_SERVICE=mathesis -- mathesis-contribute mcp
```

The NyxID mode delegates transport and credential refresh to `nyxid proxy
request`. JSON bodies go through stdin; there is no shell or copied token in
the MCP configuration. The connected service must preserve the calling NyxID
identity; see [connection requirements](../../docs/RESEARCH_SERVICES.md#nyxid-connection).
This mode takes precedence over direct URL/token settings.

For direct public access set `MATHESIS_URL=https://<venue>`; add a NyxID access
token in `MATHESIS_TOKEN` for private/mutation operations. Public discovery does
not require a token. Do not commit token values into MCP configuration files.

The MCP tools include `papers` (query, author, MSC, kind, limit, cursor), `paper`,
`review`, `knowledge-response` (submission `id` and saved `receipt`), `submit-formalization`, `formalization`, `verify-formalization`, plus
existing conjecture, target, attempt, error-report and leased contribution
tools. Reading a review requires author/staff identity. A formalization deposit
is private and unverified until the ordinary verifier checks a confirmed target.

See [API input shapes](../../docs/API.md#research-services) and
[verification and contribution rules](../../docs/CONTRIBUTE.md).

## Mathesis participation tools

The binary is now `mathesis-contribute`. Prefer `MATHESIS_URL`, `MATHESIS_TOKEN`
or `MATHESIS_NYXID_SERVICE`; legacy WISHPOOL variables remain supported.

```sh
mathesis-contribute formalization-opportunities
mathesis-contribute formalization-opportunities 'prime gaps'
mathesis-contribute formalizations
mathesis-contribute formalization-project <deposit-id>
mathesis-contribute submit-formalization project.json
```

MCP also exposes `formalization-opportunities` with q/kind/status and cursor
pagination, `formalizations`, and `formalization-project`. Intake JSON may carry
`modules` (up to 512 dotted module paths, 8 MiB total Lean source), `formalizers`
and a full `source_revision`. Original research authors and Lean contributors
remain distinct. Use `expected_target` on `verify-formalization` to bind the
request to the target you inspected. Sources become public only through the
existing independently verified attempt path.

## Review responses and verified evidence

```sh
mathesis-contribute review SUBMISSION_ID > review.json
mathesis-contribute respond-review SUBMISSION_ID response.json
mathesis-contribute verification-bundle WP-2026-0001 C1 ATTEMPT_ID > bundle.json
jq '.verification_request' bundle.json > verification-request.json
WISHPOOL_LEAN_WORKSPACE=/path/to/prepared-workspace mathesis-verifier < verification-request.json
```

`response.json` contains `round`, zero-based `concern`, current confirmed `version`,
`claims_revision`, and `body` (1–10,000 characters). Responses request a new private
review; they do not mark findings resolved. The same `respond-review` MCP tool takes
`id` and a nested `response` object. The `verification-bundle` MCP tool takes `record`,
`claim`, `attempt`. Bundles include all supporting sources, attribution and receipts;
only current public verified attempts are available. Rechecking requires a prepared
matching Lean/Mathlib workspace. The historical verifier binary revision is not recorded.


Full projects preserve `Research.Lemmas` as `Research/Lemmas.lean`. Include the
root `lean-toolchain`, `lake-manifest.json` and `lakefile.lean` / `lakefile.toml`
in `project_files`; they are stored as inert text (1 MiB total), not executed.
A source-only project can use an empty `solution`. Run
`preflight-formalization deposit.json`, then `submit-formalization deposit.json`.
To connect a saved project to an author-confirmed target, use
`verify-formalization <id> <record> <claim> Solution.lean`.
MCP exposes the same preflight as `{project: <deposit body>}` and accepts an optional
`solution` adapter on `verify-formalization`. Always pass `expected_target` in MCP.

Paper computational evidence is included in the same source ZIP/tar archive via
`mathesis-evidence.json`; see [project and evidence intake](../../docs/PROJECT_AND_EVIDENCE_INTAKE.md).
The private review dossier includes `evidence_by_version` with server-derived hashes.

Versioned manuscript delivery is also supported through `deliver-manuscript`,
`find-manuscript`, `manuscript-status` and `confirm-manuscript` in both CLI and MCP.
See [the downstream delivery contract](../../docs/MANUSCRIPT_DELIVERY.md).
The optional `--confirm claims.json` uses already-approved exact statement
confirmations; it does not infer consent or publication acceptance.

## Finite proof work and incremental contributions

```sh
mathesis-contribute proof-work RECORD CLAIM
mathesis-contribute control-proof-work RECORD CLAIM control.json
mathesis-contribute checked-propositions RECORD CLAIM
mathesis-contribute contribute-proposition RECORD CLAIM proposition.json
mathesis-contribute reuse-proof RECORD CLAIM reuse.json
mathesis-contribute result-uses RECORD CLAIM
mathesis-contribute contribution-bundle ATTEMPT > proposition-bundle.json
```

MCP exposes the same names and Layer 2 permissions. Control JSON contains
`target_digest`, `total_generations` (1–20), `expires_at` (within 30 days) and
`paused`. Pause/resume retains consumed generations and expiry; extensions are
explicit. Only the submitting author controls approved public paper work.

Proposition JSON contains `expected_target`, `intermediate` with `name`, `title`,
`proposition`, `license`, and self-contained `modules` (or an owned `deposit`).
It submits an ordinary verifier attempt; it cannot supply a receipt or successful
verdict. Reuse JSON contains `expected_target`, `solution`, `contributions` (1–8
checked attempt identities), optional new `modules`, `as_agent` and `note`.
Original modules, source pins, license and credit remain intact. A checked helper
can survive an unfinished source goal. Only actual checked proof-term occurrences
produce result-use links. No new leaderboard points or retrospective evidence.

A control request that changes an existing total or expiry must explicitly set
`change_allowance: true`. Pause/resume sends `change_allowance: false` and the
saved limits; a stale request with different limits returns a conflict. The
first grant defines its limits. Consumed generations are always preserved.
Rebinding an allowance to a corrected confirmed target also requires
`change_allowance: true`. It keeps consumed generations and saved work under a
fresh round identity; pause/resume cannot authorize the replacement target.
