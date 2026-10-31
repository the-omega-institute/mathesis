# Contribution history and result connections

Mathesis consumes existing results. A Trureturing contribution retains its
original identity, contributors, evidence and source revision. Receiving a note,
indexing a result or linking a proof does not create another mathematical
contribution. A formalization already published upstream can be presented as
source evidence without asking its authors to reproduce it for display.

## Sisyphus investigation

Inspected through GitHub at
[`ChronoAIProject/Sisyphus`, revision a54987d](https://github.com/ChronoAIProject/Sisyphus/tree/a54987d0e3bae4c92a720f19f7a262b04e832c7f),
2026-10-31. The repository is Apache-2.0 licensed. No source code was copied.

Useful foundations:

- [Typed graph nodes and edges](https://github.com/ChronoAIProject/Sisyphus/blob/a54987d0e3bae4c92a720f19f7a262b04e832c7f/sisyphus-web/src/types/graph.ts)
  keep node identity separate from display labels.
- [Snapshot, detail and neighborhood reads](https://github.com/ChronoAIProject/Sisyphus/blob/a54987d0e3bae4c92a720f19f7a262b04e832c7f/sisyphus-web/src/services/graph-service.ts)
  support selecting a result and expanding its immediate connections.
- [Mathematical node kinds](https://github.com/ChronoAIProject/Sisyphus/blob/a54987d0e3bae4c92a720f19f7a262b04e832c7f/sisyphus-api/src/papers/types.ts)
  distinguish theorem, lemma, definition, proof and conjecture.

Boundaries that matter for Mathesis:

- Sisyphus's generic `createdBy` is a data creator. Its
  [ingest history](https://github.com/ChronoAIProject/Sisyphus/blob/a54987d0e3bae4c92a720f19f7a262b04e832c7f/sisyphus-api/src/ingest/controllers/IngestController.ts)
  currently records `user: "system"`. Neither establishes mathematical authorship.
- `references` covers citation/dependency broadly; it cannot establish precise
  proof use or discharged hypotheses.
- Its [topological sort](https://github.com/ChronoAIProject/Sisyphus/blob/a54987d0e3bae4c92a720f19f7a262b04e832c7f/sisyphus-api/src/papers/topological-sort.ts)
  falls back to type ordering on cycles. Mathesis rejects cyclic contribution
  history at admission instead of presenting a false DAG.
- Its 3D force graph is not needed for reading a theorem. Mathesis uses named
  document links and a small immediate neighborhood with a mobile list layout.
- Mathesis's immutable accepted records and existing storage remain authoritative;
  this integration introduces no graph service, Neo4j dependency or upstream change.

## Implemented downstream representation

An accepted public result has its normal stable Mathesis citation, its containing
paper and the manuscript's declared prerequisite neighborhood. These relations
are explicitly labelled **Dependencies stated in the manuscript**. They do not
become Lean-certified proof-use edges.

The existing manuscript-delivery `origin` optionally includes `contributions`.
Each source contribution contains:

- its original `id`, scoped by the original source association;
- `claim_label`, matching an extracted manuscript statement;
- `role`: `problem`, `mathematical_proof`, `exposition`, `formalization` or `correction`;
- ordered `contributors`, with a name and optional public `profile_url`;
- `evidence_url` and a precise `scope`;
- an optional full `source_revision`, required for formalization and present in
  its exact source URL;
- optional `depends_on` source-contribution IDs, preserving a joint predecessor
  set. These are source-history links, not assertions that each predecessor
  independently implies the mathematical result.

At most 100 contributions are admitted per delivery. Duplicate IDs, unknown
predecessors, cycles, missing names and unrecognized statement associations are
refused. Names are never inferred from the uploader, repository commits or
profile ownership. Contributor profile URLs are optional; names alone do not
merge identities.

These records are source-declared attribution. They are retained with the
immutable accepted source and displayed as **Contributions attributed in the
original source**. They do not mint Mathesis proof/correction credit, override
paper authors, or establish upstream certificate validity. Source-based Lean
links remain separate from the existing independently checked Mathesis attempt
and its contributor attribution. Corrections confirmed in Mathesis continue
through its existing error-report path.

The result page links to original research, displays the exact scoped source
credits, and lets the reader open evidence for each contribution and its direct
predecessors. It exposes no internal execution stages or hashes by default.
The existing API serves this same data; it is not private frontend state.

## Compatibility and privacy

An omitted/empty contribution list retains the exact legacy delivery JSON
encoding, so replaying an existing delivery does not change its request digest.
Old papers still show their existing author credit, result connections, proof
attribution and supplied source links. Missing step-by-step credit is left absent.
A new private revision cannot rewrite a published contribution history. Switching
an accepted record to private hides its origin and detailed result graph too.
No test publication or real-manuscript publication is needed to validate this.

## Remaining integration boundaries

This is the downstream storage and presentation of actual contribution records,
not an assertion that all upstream results have already been synchronized.
The current Outreach `outreach-mathesis-handoff.v1` preparation package must be
translated into the existing `mathesis.manuscript-delivery.v1` envelope by the
Mathesis consumer; missing source attribution must remain missing. No upstream
code or publication contract needs changing to support older deliveries.

General cross-paper entailment or minimal-dependency claims still require the exact statement identities,
proof location, substitutions, hypothesis discharge and public evidence described
in [RESEARCH_EXPERIENCE.md](RESEARCH_EXPERIENCE.md). Module imports, search matches,
bibliographic citations, and source-history edges are not sufficient. A verified
contribution graph and a Lean proof dependency graph are different projections;
we must not collapse them into an unlabeled network.

## Reading a result

1. Open a theorem from its paper; its statement and original paper authors stay
   first, followed by its citation.
2. Follow a named prerequisite or later use to inspect that result's own direct
   neighborhood. Open assumptions remain explicitly conditional.
3. Read **Source and contributions** for the source's credited work, with an
   evidence link at every recorded step. Multiple predecessors remain visible.
4. Open an existing upstream Lean proof directly. A separate Mathesis-checked
   proof appears only when the independent verifier has its own receipt.

Ordinary authors do not edit a graph during submission. Missing historical
attribution is not generated from names, model guesses or service execution logs.

## Independently checked incremental work

Named auxiliary propositions now live in the existing SolveFile/Attempt aggregate.
They are separately checked against an exact type and theorem declared in submitted
modules. Their source result can remain unfinished. Checked proposition exports
retain source version, claims revision, target/project pins, source authors,
contributor, optional deposit formalizers, original license and receipt scope.
No second credit ledger or new score is introduced.

A proof may select up to eight originals and reuse their exact modules. A trusted
verifier inspection follows elaborated proof values in the submitted context;
only reached selected constants create `checked_proof_term` links. Both result
pages expose the links, source evidence and complete selected joint context.
The relationship concerns the contributed proposition, not a blanket claim that
the source manuscript's final theorem supplies the whole proof. Unused imports,
citations and source-attributed histories produce no checked-use links.
Visibility and immutable accepted inputs are checked at both endpoints, including
on profiles, counts and downloads. See [LEAN_REVIEW_EXPERIENCE.md](LEAN_REVIEW_EXPERIENCE.md).
