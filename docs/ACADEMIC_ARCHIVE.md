# Academic archive and author report

The arXiv references for this change are its
[abstract-page structure](https://arxiv.org/abs/2303.12712) and official
[submission guidance](https://info.arxiv.org/help/submit/index.html), inspected
on 30 October 2026. The relevant conventions are an author-and-abstract-first
record, direct PDF/source access, subject navigation, explicit version history,
ordinary citations/BibTeX, reference-manager metadata, and checking processed
files and bibliographic metadata before final submission.

## Delivered behavior

- The home page introduces review and participation, followed by recent work and
  a short, visible subject directory. It has no subject accordion or search form.
  Existing upload-first tabs remain Upload, Check PDF, Details, Authors, Submit.
- `/subjects` lists the 63 two-digit MSC 2020 classes. `/subjects/:msc` opens a
  named subject's recent publications directly, with ordinary bibliography entries
  and pagination. It does not open the archive search form. Invalid classes have
  a not-found page; empty classes say that no papers have been listed yet.
- Both publication paths participate in subject browsing. Published notes without
  supplied MSC metadata use Mathesis's broad, title-and-abstract-based indexing in
  `web/src/lib/projectNoteSubjects.ts`. Supplied version metadata takes precedence;
  an unclassified future note remains in the full archive. Indexing does not create
  a review, acceptance decision, or formal-verification claim.
- Archive queries retain text, author, MSC and type in the URL. Author-name links
  use this search; existing MSC search URLs still work. External academic profiles remain
  separate. Name search does not claim a unique researcher identity.
- The public abstract page places files with the title and authors. Abstract,
  subjects, DOI/arXiv, versions and a folded citation follow. Source and PDF links
  name an explicit accepted version; PDF-only intake does not advertise TeX.
  The API's optional publication identity is omitted for private records.
- Plain-text and BibTeX export cite the actual authors and work, the Mathesis
  acceptance record and existing identifiers. Unknown venues/years/DOIs are
  not invented. BibTeX keeps balanced mathematical TeX and contains malformed
  metadata. Standard citation metadata supports browser reference managers.
- Every public accepted statement has `/papers/:record/results/:claim`, even
  before a Lean target exists. It includes mathematical authors, the full
  statement, source PDF and a result citation naming the publication version.
  Formalization contributors keep separate credit. Within-paper links show
  direct prerequisites and uses as declared in the manuscript; open dependencies
  are explicitly conditional. The accepted source stays fixed across private
  manuscript revisions and reused local claim identifiers.
- New report contract 2 uses Review analysis and Further directions. The
  application appends selected Formalization opportunities after actual letter
  delivery, from a bounded correct-claim projection. One readable download
  includes the same recommendations. Existing letters and running contracts
  remain unchanged. Recommendations do not create an approval, task or proof.
  Author controls no longer pretend every proved statement was recommended by
  an editor; additional requests are a deliberate, folded action.
- Feedback delivered before a publication decision remains a folded secondary
  section explicitly marked decision pending. It is not labelled a final report.
- Legacy generic result labels use their manuscript section and stable claim
  identifier instead of repeated unnamed headings. Review reports render both
  dollar and conventional TeX math delimiters, preserving code examples and the
  original stored/downloaded report text.

## Boundaries

This applies arXiv's manuscript-facing conventions to Mathesis's own review and
publication policy. It does not add human endorsement, moderator roles or extra
submission acknowledgements. No current/private manuscript becomes public, no
review is restarted and no upstream repository is changed by this release.

The general cross-paper knowledge graph in
[the research experience design](RESEARCH_EXPERIENCE.md) remains a separate
extension. This release provides stable result citations and truthful local
connections; it does not present source imports, bibliography matches, author
assertions or private audit findings as publicly verified cross-paper uses.

## Verification

Check the public abstract/result pages, source and PDF version links, BibTeX
copy/download, search URL restoration and subject/author navigation at desktop
and mobile widths. Check author views for accepted, not accepted, private and
pending cases, including actual delivered letters and absent recommendations.

The domain tests exercise delivery gating and bounded author projection, the
immutable publication across private revisions, and unchanged legacy prompts.
Web tests cover citation escaping/provenance, public-only result access, conditional
assumptions, search URLs and the corresponding page behavior. Browser fixtures
remain local and are never submitted to production review or publication.

## Paper presentation, 6 November 2026

The paper list and abstract pages also draw on [Hexagon](https://hexagonmath.org/)'s
separation of bibliographic content, file access and supporting metadata. Archive
rows remain compact: title, authors and version/record occupy the main column;
dates and available file links align in a secondary column. Full abstracts and
reading guides stay on the work page. Small screens stack each row in reading order.

Both paper readers share title typography, an abstract-first main reading column,
section links, and a sidebar for publication metadata, citation and versions.
Navigation opens folded citations and related sources when selected. Complete
plain-language explanations follow the global language setting. Shared site
margins, manuscript authorship, exact-version links, private-source restrictions
and the distinction between Lean sources and checked proofs are preserved.
This presentation adds no admission, review or formalization work.
