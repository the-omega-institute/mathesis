# Upload, inspect, describe, submit

The manuscript workflow follows the order documented in the [arXiv submission overview](https://info.arxiv.org/help/submit/index.html): upload, process the source, inspect the PDF, then check bibliographic metadata and submit. Mathesis uses its own AI review and publication policy.

1. `POST /api/v1/submissions/prepare` accepts multipart `metadata` and `source`, or JSON `{metadata, filename, source_base64}`. Metadata contains `kind`, `purpose` and `processing_consent: ai-review-request-2026-10-29` for the upload action (legacy `ai-review-2026-10-11` is also accepted). PDF, TeX, ZIP and tar.gz have the ordinary 30 MiB limit.
2. Preparation saves a private draft with `intake.confirmed_request: null`. Only the durable compile job is queued. `GET /api/v1/submissions/{id}` returns compile progress and source-derived title, abstract and authors. Missing/ambiguous metadata remains empty. PDF extraction is heuristic and does not infer authors from affiliation/date lines.
3. Inspect `/api/v1/submissions/{id}/files/pdf?version=N`. Correct any source error with the existing version-upload route. The browser resumes incomplete drafts from My work. Metadata edits are kept while moving between tabs, and are persisted on final submission.
4. `POST /api/v1/submissions/{id}/submit` accepts `{version, source_sha256, claims_revision, metadata, claims}`. `metadata` is the ordinary `NewPaper` shape with checked title/authors/disclosure/visibility and the same manuscript kind. The author chooses the final `purpose` at this point; it may differ from the provisional preference saved during preparation. `claims` are the author's exact statement confirmations. This writes metadata and confirmation together under the aggregate revision fence, then queues ordinary review. It requires the prepared PDF, submitting owner and matching current source; exact final retries return the existing submission without another confirmation or review round.

The web form has no paper/short-note selector. Default file intake uses paper review; a source containing only open statements is prepared as a conjecture. Existing explicitly typed notes remain supported. Clicking “Continue to details” after the PDF is ready advances to metadata. The Authors step directly exposes affiliation, ORCID, Google Scholar and a website or other link for each author; profiles use the existing validated author metadata and accepted-record display. No draft-save/My work reminder appears under the form.

The web form defaults to “Make my results public once accepted.” in the final step. Unchecking it submits `purpose: review_only` with `make_public_after_acceptance: false`; it requests private feedback without a publication decision or record. Existing review-only drafts and legacy review links keep that checkbox initially unchecked. Community formalization has its own entry; manuscript intake has no repeated type-selector row.

Preparation grants no publication, contributor access, claimed authorship or AI verdict. Ordinary claim confirmation cannot skip incomplete metadata. Private review stays private. Final publication preference still takes effect only through acceptance. Existing submissions and producer manuscript deliveries are unchanged.

CLI and MCP expose the same operations:

```sh
mathesis-contribute prepare-submission preparation.json manuscript.zip
mathesis-contribute submission SUBMISSION
mathesis-contribute submit-prepared SUBMISSION confirmation.json
```

MCP uses `prepare-submission` with `metadata`, `filename`, `source_base64`; `submission` with `id`; and `submit-prepared` with `id`, `confirmation`. The upload action prepares the manuscript; the final submit action confirms the source, metadata and statements and requests AI review. No separate acknowledgement checkbox is needed. Clients must act on an author’s request, and must not fabricate a legacy checkbox receipt. For durable producer/work deduplication, use the separate [manuscript delivery contract](MANUSCRIPT_DELIVERY.md).
