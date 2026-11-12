# §19. Academic submission interface

Part of the repository contract in [CLAUDE.md](../../CLAUDE.md); equally binding.

- Stable source problems group accepted notes through existing `origin.problem_ids`.
  Optional `origin.problems` carries actual original statements, posers and claim
  mappings. Public relationship reads use accepted public snapshots only. An
  association or source-declared outcome never substitutes for the existing exact
  source-audited settlement or independent Lean verification. Applying a later
  acceptance advances a separate snapshot on the submission; the original Record
  stays immutable. Pending versions and recompilation cannot replace accepted
  inputs. See `docs/PROBLEMS_AND_NOTES.md`.

- People is an administrator account list, with only an Administrator access switch.
  Legacy roles remain readable for historical access; the interface neither advertises
  nor grants editor, reviewer or endorser roles. The account menu omits editorial labels.
- Use the archive's page alignment, horizontal submission steps, readable text measures
  and restrained rules. Avoid nested centred cards, rounded status pills or decorative
  borders. Keep real input boundaries, keyboard focus, error text and responsive layout.
- Public pages share the header/footer's outer margins through `--page` and `--gutter`.
  Keep headings and page containers on those same edges. Restrict prose within the
  shared page instead of centering narrower page wrappers or widening individual routes.
- Public paper lists keep compact bibliographic rows, with dates and file access in
  a consistent secondary column on desktop. Both paper readers use the same title,
  abstract-first reading order, section links and citation/version sidebar; narrow
  screens retain document order. Keep complete bilingual explanations on the work
  page and show only sections and public downloads that actually exist.
- See `docs/UPLOAD_FIRST_SUBMISSION.md` for draft preparation and client interfaces.

- A manuscript in `in_review` cannot receive a new source version, including through
  producer deliveries. Revision upload returns a conflict before source/blob/queue
  changes. Exact delivery/confirmation retries retain their existing idempotence.
- Throughout `in_review`, even after an internal letter is delivered, the author page
  contains manuscript metadata, PDF/source links, a short status and a folded
  withdrawal control. Statements, execution history, compute settings, research
  goals and downstream proposals are absent. Staff retain the separate execution view.
  Public binding tools require accepted public work with contributor access;
  knowledge proposals require accepted public work; research goals follow report delivery.

- Author and public pages present manuscript content, the decision, one final report,
  contributor credit and optional formalization. Internal stages, execution timers,
  hashes and raw verification JSON belong in staff views or explicit downloads.
  Manuscript binding/proposal/research panels and per-review compute history are staff-view
  tools; this UI choice never expands API authority or substitutes for owner consent.
  Author details retain source versions, supplements, visibility and formalization controls.
  Developer integration instructions are collapsed on the contribution page. The public
  review policy explains mathematical criteria and privacy without internal stage names.

- New rounds pin `letter_contract: 3` for Review analysis, Suggestions and Further
  directions. Contracts 0/1/2 retain their prompt bytes. The existing
  venue-authored decision remains authoritative. After same-round letter delivery,
  a bounded projection exposes selected correct-statement formalization plans;
  it never exposes raw advice, sketches, files or provider data. Recommendations
  are separate from approvals and checked proofs. Pending decisions expose no author report,
  report download, results list or revision control. Staff retain the original letters.
  Completed reports use a fixed opening from the applied state and visibility, never
  from a model recommendation. Legacy mathematical analysis is drawn from the same-round
  audit and delivered research suggestions, without displaying the old status prose.
  Future reports use the three mathematical sections; malformed sections fall back to
  the saved findings. The rendered and downloaded reports are identical. Author/public
  references use manuscript names, printed numbers or sections, never internal claim IDs;
  literal mathematics, code and source URLs remain unchanged.
- Public abstract pages include pinned PDF/source downloads, subject links,
  version history, plain-text/BibTeX citations and reference-manager metadata.
  Named result pages use the immutable accepted record even without a Lean target.
  Within-paper dependencies retain their manuscript-declared basis; open assumptions
  are explicitly conditional. No bibliography citation, similarity or local edge
  is promoted to an independently checked cross-paper dependency.
- An accepted public author paper exposes its archive descriptor only when the current
  accepted source version carries the exact archive-consent marker. The descriptor names
  the CC BY 4.0 licence, consent time and marker, verified exact and concept DOIs when
  present, and byte size, SHA-256 and download URL for the frozen PDF and source. The
  source route returns not found for every other state. Public listing rows carry the
  current-version consent fact. Reviews, audits, letters, advice and other-version
  consent never enter this projection.
- Archive queries retain text, author, MSC and work-type filters in shareable URLs.
  Author-name links search the archive; supplied ORCID/Scholar/site links remain
  distinct, without treating a name match as a verified personal identity.
  See `docs/ACADEMIC_ARCHIVE.md` for the arXiv reference and delivered scope.

- A delivered analysis without a publication decision is labelled Awaiting decision,
  with a brief mathematical reason such as a remaining comparison with prior work.
  It must not imply that a completed execution is still running automatically.
  Manuscript and submission pages use ordinary review language; venue-level AI
  explanations belong on the homepage and policy, while author-supplied disclosures
  remain part of the manuscript metadata.
- Completed reports initially show a concise overall assessment, up to three useful
  improvements and two proposed directions, and a short formalization status. Keep
  full findings and plans in a closed Detailed report disclosure and include them in
  the single download. Never truncate mathematical sentences or treat an excerpt as
  the full report. Preserve named/numbered results; when a legacy result lacks a title or number,
  use its result type without replacing it with a section heading or inventing a number.
