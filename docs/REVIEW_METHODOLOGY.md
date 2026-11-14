# Review methodology: new content and mathematical value

New automatic referee rounds and independent paper-problem reviews pin contract
3, `trureturing-content-2026-10-15`. The method follows Trureturing's proof-escape
and computational-utility rules, using the existing Oracle referee → CMA source
auditor → deterministic domain policy → advice/letter pipeline. It does not
import Trureturing's theorem corpus, compiled judge or build caches.

The reference is Trureturing `CLAUDE.md` §§3.2, 3.3 and 3.9, and
`docs/develop/spec/auric_fib_atom_escape_audit_spec.md` §1, inspected at revision
[`b6577be7be6daa94c9bb1db4628acb5b5bc71ebc`](https://github.com/the-omega-institute/trureturing/tree/b6577be7be6daa94c9bb1db4628acb5b5bc71ebc)
on 2026-10-15. In particular, the opt-in FIB analysis from
[PR #16176](https://github.com/the-omega-institute/trureturing/pull/16176)
does not confer proof escape, admission or registration authority.

## Separate judgments

| Dimension | Required evidence | Meaning |
|---|---|---|
| Correctness | Source proof, hypotheses and identified gaps | `correct`, `gap`, `error`, `not_checked`; unchanged and never a Lean receipt |
| Proof escape | Explicit prerequisites, binding comparison, exact delta and live witnesses | `content`, `bind_only`, `undetermined` |
| Literature comparison | Actual queries/scope and inspected prerequisite sources | `supported`, `unsupported`, `insufficient`; a bounded reviewer judgment, not exhaustive novelty certification |
| Utility | Mathematical question answered or category-specific use, with limits | Independent from proof shape; no scalar value score |
| Information gain | Optional model, readout, task, faithful source correspondence and evidence | Reported analysis only; never an admission gate or a formal FIB proof |

Mathematical proofs derive conclusions from premises. Escape does not demand
logical independence from those premises. The comparison asks what existing
results already yield by instantiation, assumption substitution, projection,
recombination and normalization, and identifies the step that exceeds those
operations. New aliases and helper definitions are inlined for this comparison.
Existing prerequisites form the stopping boundary.

A witness may be an intermediate proposition or the conclusion itself, provided
it is produced by a non-binding construction, computation or estimate and lies
on the live deduction path. Every witness records its statement, source location,
non-binding step and actual use. An unused lemma does not count. A consumed
bind-only helper is useful proof organization but is not independent new content.

## Structured audit and admission

Every contract 3 audited claim has `methodology`:

- `baseline`: statements with hypotheses, locations and optional model-reported
  opened external sources. Manuscript-local prerequisites use `source: null`.
- `searched`, `literature`: actual search scope and adequacy of the comparison.
- `binding_analysis`, `delta`, `escape`, `witnesses`: proof-escape evidence.
- `utility`: category, evidence status, rationale, independent problem, progress,
  exact prerequisites and their discharge.
- `settlement`: optional exact confirmed problem name/source, original statement,
  assumptions, prior open status, resolution, matching argument and opened source.
- `information_gain`: optional separate analysis; normally null.
- `limits`: unresolved obligations and boundaries of the evidence.

The binary bounds and validates untrusted model output, preserves the existing
opened-source syntax checks, and projects structured witness statements into the
existing witness list. A missing or malformed methodology becomes an explicit
insufficient/undetermined reading. It does not silently become legacy acceptance,
binding, a citation, or an invented witness. Source-proved correctness remains
separate from the missing novelty/value evidence. Domain admission requires the
new object for every statement in a new round and checks structural consistency.

The new-content route requires correct main-result content, no matching prior
work, a supported comparison, and supported utility. Pure general mathematics
does not need an immediate application or manufactured downstream consumer.
Positive finite enumeration and ordinary certified positive instances alone
remain supporting evidence. Checkers require soundness and an actual independent
use; numerical reductions must discharge their exact numerical premises and
have a concrete consumer or refutation. A counterexample needs its named
assertion, matching hypotheses and refutation, without an extra consumer.

The separate settlement route can admit a bind-only proof after the audit
matches the exact confirmed named open problem and its source. The author's
`settles` field alone cannot grant this route. Partial answers or changed
hypotheses do not establish settlement. Missing evidence yields
`insufficient_evidence`; unsupported computational usefulness yields
`insufficient_utility`. These reasons do not assert that the work is worthless.
Existing stage, correctness and endorsement gates still apply. Any incorrect,
gapped or unchecked main result blocks publication as before.

Conjectures retain their well-posed/open/content test and add the same baseline,
search and utility discipline. Their delta describes a conditional new obligation;
they carry no live proof witnesses and no proof-correctness assertion. Independent
paper-derived problem reviews use the same versioned protocol.

## Compatibility, privacy and limitations

Each round already binds version, claims revision and source digest. Author-safe
JSON and Markdown exports now also identify its review contract and method, and
expose structured evidence privately to authors/co-authors/staff. No private
comparison, utility judgment or source audit is added to public publication views.
Public witnesses remain restricted to audited-correct content results.

Contracts 0–2 keep their original prompt bytes, optional field omissions and
admitted remote task identities. Existing reports without `methodology` retain
their old policy meaning. Accepted records are not rewritten. A new review round
uses contract 3; no automatic migration/backfill is performed. JSON dossier schema
2 receives additive fields; clients must handle the new `undetermined` proof shape
and rejection variants. Human/editor legacy stage-report and corroborated
judgment routes remain available under their existing authorization rules.

Validation checks structure and consistency, not the truth of a free-text
argument. A model can still misread a source, miss a prior result or overstate
value. The reports expose those grounds for review and response; they are not
compiled Trureturing registrations or independently verified Lean receipts.
An optional information-gain narrative is not formal AURIC/FIB validation.
Existing `EscapeRateReading` remains separate and requires a built, measured
finite arena; no universal novelty percentage is introduced.

This change does not provision a CMA sandbox, enable live model service or alter
identity/payer selection. Deployment still requires usable company-managed CMA
resources and Oracle access. GitHub Pages is a separate static project showcase;
the actual review application needs its backend and database.

## Advice: value and checked targets

Advice gives the advisor's reading of what a specialist learns and the single
weakest point, consistently with the supplied audit. If main results are known,
it says what remains. The manuscript's novelty reading stays with the referee and
source audit; advice neither repeats that verdict nor restates or changes the decision.

Each proposed or newly proved strengthening is stated with exact hypotheses,
quantifiers and conclusion, then checked against opened literature before listing.
The target's reported finding is `known`, `specialization_of`, `related` or
`not_located`, with source, theorem/equation/page and a precise comparison.
Known targets and specializations are literature connections. A nonfinding states
actual search scope and limits; it never asserts novelty. Sources are reported by
the model, never independently verified by Mathesis. Feasibility says what is in
hand and missing; risk identifies the most likely failure or prior-work overlap.
Positive ranks order value without a scalar score, with one rank-1 addition when
a grounded direction exists. Constants too weak for practical use are described
as conservative certificates, with a recommendation on proof placement.

Value stays staff-only. Delivered author research suggestions gain only the
optional ranked target fields for the improvements already selected there. The
binary bounds new strings, checks claim identities, drops invalid verdicts and
unopened evidence, and names omissions in the advice summary. Old advice JSON
omits absent new fields unchanged. Letter contract 4 adds rank-first directions
and target comparisons in the note; letter contracts 0–3 retain their instructions.
Advice instructions have no version pin: saved CMA digests reject changed inputs;
not-yet-admitted advice uses the current template. This existing behavior remains.
