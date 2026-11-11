# Collected open problems

A trusted operator can collect a curated open problem directly into the public
problem library: an OEIS conjecture, a MathOverflow question, a numbered
conjecture in a paper, and similar. The problem becomes a lasting entry with
its original poser credited, before any work addresses it. Collection is
separate from review. It records what a curator read and when; it is not a
Mathesis referee reading, audit or admission.

This channel is the sibling of [pre-reviewed collection](PRE_REVIEWED_COLLECTION.md)
and follows the same conventions. The library itself is described in
[PROBLEMS_AND_NOTES.md](PROBLEMS_AND_NOTES.md).

## Intake

`POST /api/v1/collected-problems` takes an authenticated JSON request. The
caller's identity must be listed in `WISHPOOL_INTERNAL_SUBJECTS`, exactly as for
pre-reviewed collection; anyone else receives the same `403 forbidden`
("trusted collection operator required"), and a request without a caller
receives `401`. Roles, including Administrator, do not open the channel.
There is no new configuration key.

```json
{
  "schema": "mathesis.collected-problem.v1",
  "producer": "outreach",
  "problem_id": "example-gap-conjecture",
  "version": 1,
  "previous_version": null,
  "authorize_publication": true,
  "problem": {
    "name": "The example gap conjecture (Conjecture 1.2)",
    "statement": "For every $n \\ge 1$, every gap in the sequence $a(n)$ is at most $2$.",
    "background": "Optional context, also LaTeX source text.",
    "posers": [{"name": "A. Poser", "profile_url": "https://example.org/~poser"}],
    "source": {"kind": "arxiv", "locator": "2401.12345", "year": 2024},
    "status": "open",
    "status_evidence": [
      {
        "url": "https://arxiv.org/abs/2401.12345",
        "checked_on": "2026-11-10",
        "observed": "Version 3 still states Conjecture 1.2 as open."
      }
    ],
    "msc": ["11B05"],
    "directions": ["gap-bounds"],
    "source_license": "arXiv non-exclusive distribution licence",
    "license": "CC-BY-4.0"
  }
}
```

The example is illustrative. Unknown fields are refused at every level.

| Field | Rule |
|---|---|
| `schema` | Exactly `mathesis.collected-problem.v1`. |
| `producer`, `problem_id` | Lowercase ASCII slugs (`a-z`, `0-9`, `-`), at most 160 characters. `problem_id` is the operator's stable name for the problem. |
| `version`, `previous_version` | `version` ≥ 1. The first version names no predecessor; every later one names the exact current version. |
| `authorize_publication` | Must be `true`: the entry is public as soon as it is collected. |
| `problem.name` | Required, at most 2,000 characters (the bound of a problem named by a collected work). |
| `problem.statement` | Required LaTeX source text, at most 20,000 characters. |
| `problem.background` | Optional, at most 50,000 characters. |
| `problem.posers` | Ordered original posers as the source names them: `name` (at most 300 characters) and an optional public HTTPS `profile_url`. At most 50. |
| `problem.posers_unknown` | Required exactly when `posers` is empty: why no poser is named (at most 1,000 characters). |
| `problem.source` | The existing `Source`: `kind`, `locator`, optional `year`. |
| `problem.status` | `open` or `partially_resolved`. No other value is accepted. |
| `problem.status_evidence` | 1 to 10 items `{url, checked_on, observed}`: a public HTTPS page without credentials, the calendar day (`YYYY-MM-DD`) the curator opened it, and what it showed (at most 2,000 characters). A day more than one calendar day ahead of the server's UTC date is refused. |
| `problem.msc` | Optional, at most 8 MSC codes. |
| `problem.directions` | Optional research-direction tags: at most 16 distinct lowercase ASCII slugs of at most 80 characters. |
| `problem.source_license` | Required: the source's own licence terms as the source states them (at most 500 characters). |
| `problem.license` | Licence of the curated restatement: `CC-BY-4.0`, `CC-BY-SA-4.0` or `CC0-1.0`. Share-alike is admitted because common sources are share-alike. |
| `problem.lean_target` | Optional: the curator's Lean statement of the problem, `{text, correspondence}`. See [Lean target](#lean-target). |

The existing source kinds express the intended sources without extension:
`oeis` (`A` and six digits), `arxiv`, `doi`, and `url` for a MathOverflow
question or any other web page; `named_work`, `zenodo`, `hexagon` and
`personal` keep their existing meaning.

All operator-supplied text is data. It is stored and projected as written,
rendered as text or LaTeX, and never interpreted as an instruction. Links are
rendered only for web addresses.

### Receipt, retries and versions

A successful request returns

```json
{
  "schema": "mathesis.collected-problem-receipt.v1",
  "status": "collected",
  "problem_id": "example-gap-conjecture",
  "version": 1,
  "research_problem": "<64-hex library id>",
  "public_path": "/problems/<64-hex library id>",
  "request_digest": "<sha256>"
}
```

The request digest is the SHA-256 of the parsed envelope in the server's
canonical form. A retry of the same content returns the same receipt and writes
nothing. A changed payload under a published version is refused with a
conflict. Published versions are immutable; a correction, a new evidence check
or a changed reading is a new version naming the current one. Each write is
fenced by that predecessor: MongoDB holds unique problem/version,
problem/predecessor, entry/version and entry/predecessor pairs, so a slug has
one unforked chain and concurrent deliveries cannot both succeed. A concurrent
identical delivery shares the winner's receipt.

A `problem_id` belongs to the operator and producer that first collected it.
Versions are stored in the `collected_problems` collection; no file or GridFS
blob is involved.

## Identity

The library id is the existing collected-problem key: the collecting operator,
the producer, the source kind, the source locator and the exact name.
Surrounding whitespace is ignored and `source.year` is descriptive only. A
directly collected problem and a problem named in `paper.problems` by a
pre-reviewed collection delivery are therefore the **same entry** when they
agree on all five. This is the intended loop: a curated open problem is later
addressed by a collected work that names it, and the problem page then lists
that work. The order of collection does not matter.

Nothing is merged by similar titles or statements. Another operator, producer,
source or name is another entry, and delivery problem ids remain a separate
key family.

Because name and source are the identity, every version of a `problem_id` keeps
both. A renamed or re-sourced problem is collected under a new `problem_id` and
is a new entry until separate evidence reconciles the two. One named source
question has one `problem_id`.

## Public projection

`GET /api/v1/research-problems` and `GET /api/v1/research-problems/{id}` include
a directly collected problem even when no work names it yet; `summary.notes` is
then `0`. The detail adds `curated`, the current version: `name`, `statement`,
`background`, `posers`, `posers_unknown`, `source`, `status`, `status_as_of`,
`status_evidence`, `msc`, `directions`, `source_license`, `license`, `version`,
`published_on` and, when the version carries one, `lean_target`
([Lean target](#lean-target)). Collected works naming the problem remain in
`collected_notes`. The projection names no operator, producer, `problem_id` or
request digest. Earlier versions stay stored and are not projected.

The list keeps its existing `limit`/`before` paging by stable identity order.
It has no status filter or text search, as for every other kind of entry.

## What the status means

`summary.status` is `curated_open` or `curated_partially_resolved`, with
`summary.status_as_of`: the most recent `checked_on` among the evidence. These
two values are the curator's dated reading of the listed pages. They are
distinct from `related_research`, `settled`, `partial` and `conflicting`, which
keep their meaning and come from Mathesis source audits of accepted notes.

- The API and the problem page present the reading with its date and its
  evidence, and state that it is not a Mathesis review or audit result.
- A settlement cannot be asserted through this channel.
- A collected work naming the problem does not change the status: a collected
  work carries no Mathesis source audit. The entry never becomes **Settled**
  merely because a work names it. When the curator reads new evidence, a new
  version records the new reading.
- Named posers and sources are reported, not verified identities or
  authenticated citations.

## Lean target

A version may carry the curator's own Lean statement of the problem:

```json
"lean_target": {
  "text": "import Mathlib\n\ndef wishpool_target_prop : Prop :=\n  ∀ n : ℕ, 1 ≤ n → ∃ k : ℕ, 1 ≤ k ∧ ...\n",
  "correspondence": "The products run over k consecutive integers starting at n and at n + k."
}
```

It is the collection operator's formalization. The original poser has not
confirmed it, and it carries no proof. An entry collected without a target
receives one as its next version.

| Field | Rule |
|---|---|
| `text` | The exact `Target.lean`, at most 200,000 characters. It starts with `import Mathlib` and imports nothing else. It declares `def wishpool_target_prop : Prop := <statement>` exactly once, as the last declaration of the file, and the name `wishpool_target_prop` occurs nowhere else, comments included. Definitions the statement needs stand between the import and that declaration. |
| `correspondence` | Required, at most 5,000 characters: how the Lean proposition reads the curated statement, in plain words. |

The intake refuses a target that breaks these rules with `422`. Everything else
is decided by the independent verifier:

1. Collecting a version with a target queues one durable worker job for that
   exact version (`collected_problem_target`, in the verification lane). An
   exact retry of the delivery queues nothing once the check is stored.
2. The job asks the verifier service (`WISHPOOL_VERIFIER_URL`) to check an
   **elaboration witness**. For the curator's proposition `p`, the witness
   target declares `wishpool_target_prop` as `p → p` behind the same
   definitions, and the witness solution is the identity function. The
   verifier accepts it exactly when the definitions and `p` elaborate as a
   proposition in its pinned Lean and Mathlib, under its ordinary rules for
   targets (no proof hole, no axiom, no unsafe code, no other import). The
   proposition itself is never claimed to hold. The witness keeps the line
   numbers of the curator's text, so a refusal cites the curator's own lines.
3. The outcome is attached once to that version: `elaborated`, or `refused`
   with the verifier's reason. A deadline or an unreachable verifier writes
   nothing and the job runs again. A refused target is corrected by a new
   version; versions stay immutable.

The public projection `curated.lean_target` carries:

| Field | Meaning |
|---|---|
| `state` | `checking` until the verifier has read this exact text, then `elaborated` or `refused`. |
| `digest` | SHA-256 of the exact target text. |
| `correspondence` | The curator's account, as delivered. |
| `vocabulary` | `mathlib` when nothing between the import and the proposition declares a constant: only blank lines, comments and `open`, `namespace`, `section`, `end`, `universe` or `variable` commands. Otherwise `own_definitions`. The reading is textual and errs towards `own_definitions`. |
| `text` | The Lean text, present only in state `elaborated`. |
| `toolchain`, `checked_at` | The verifier's pinned Lean and Mathlib and the time of its reading, once it has read the text. |
| `reason` | The verifier's reason, present only in state `refused`. |

What an elaborated target establishes, and what it does not:

- The text is a well-formed Lean proposition in the pinned environment, using
  Mathlib and its own definitions only.
- It does not establish that the proposition states the problem faithfully.
  That reading is the curator's, and the page says so.
- It opens no proof attempt, verification of a solution, credit or leaderboard
  entry. A curated target is not a confirmed public target in the sense of
  [contract §12](contract/12-solving.md); only author-confirmed targets of
  reviewed conjectures are.
- `vocabulary` reports whether the statement can be written with what Mathlib
  already defines. It is not a search for an equivalent statement in a
  prepared library.

## Web

**Conjectures → Research record** lists curated entries as "Curated · Open as
of <date>" (or "Partially resolved as of <date>"). The home page path **Work on
an open problem** opens that list. The problem page `/problems/{id}` renders
the statement as mathematics, the posers, the source link, the dated reading
with its evidence list, the curator's Lean statement when the version carries
one, and two existing ways to take part:

- **Submit a written solution** opens the ordinary submission path and its
  ordinary review.
- **Contribute a Lean formalization** opens the existing Lean contribution
  intake as `/submit?type=lean&problem={id}`, with the name, statement,
  original posers and problem page prefilled, as pending results already
  prefill it. The deposit is the ordinary private source deposit.

## What the channel does not do

- No referee, Oracle, source audit, admission record, `WP-` record id or
  publication decision. `Policy::decide` is not involved.
- No proof job, verification of a solution, credit or leaderboard change. A
  Lean target is delivered by the curator, never generated: the channel runs
  no model to write one, and a Lean deposit for a curated problem is not
  verified against it.
- No reading guide or other model work. The only queued job is the verifier's
  reading of a delivered Lean target.
- No import: a problem is collected only when an operator calls the endpoint.
  Nothing scrapes a source, and nothing is submitted to any deployment by the
  code itself.
- No check that an evidence page says what the curator observed. Mathesis
  does not open the URLs.
- No operator CLI or MCP command, as for pre-reviewed collection. Contributor
  CLI and MCP read curated entries through the existing `research-problems`
  and `research-problem` commands.
