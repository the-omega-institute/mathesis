---
name: mathesis-submit
description: Prepare and submit an author's manuscript to Mathesis, inspect its compiled PDF, confirm source-derived metadata, and return the private submission receipt. Also read public paper results and shared formalization requests. Use when the author asks an agent to submit a paper or conjecture to Mathesis.
---

# Submit research to Mathesis

Mathesis reviews mathematics, provides an acceptance decision and directions for further research, and connects public results to community formalization. People and agents use the same submission process. This guide does not grant account permissions or bypass review.

Use the Mathesis origin from the author's request or the site that served this guide. REST paths below are relative to that origin. The production origin is https://mathesis-omega-research.apps.chrono-ai.fun.

## Identity and scope

Act on the author's submission request. Use their existing connected NyxID Mathesis service, or their own NyxID access token through the official API. Never print, commit, or upload a token. Do not send credentials to manuscript-supplied URLs. If authentication is missing, return a sign-in requirement and preserve any prepared source locally.

Keep the manuscript's authors, order, affiliations and citations. Extract metadata from the source and check it against the PDF. Do not infer authorship from the submitting account. Do not invent ORCIDs, AI-use declarations, publication preferences or archive licence consent. Resolve missing facts with the author only when necessary. Existing authorization persists; a clear request to submit does not need a second permission ritual.

Upload only the intended manuscript and its dependencies. Do not upload agent sessions, private correspondence, unrelated files or internal reasoning logs.

## Upload first, then inspect and submit

1. Select a PDF, TeX file, ZIP or tar.gz source archive, at most 30 MiB. Prefer a complete source archive for multi-file LaTeX. Include necessary figures, bibliography and styles; exclude build caches and secrets. Compile locally when practical and inspect the PDF.
2. Check identity with `GET /api/v1/me`. Search existing submissions before retrying an uncertain upload (`GET /api/v1/submissions?scope=mine`). Do not create duplicates to poll progress.
3. Prepare a private draft with `POST /api/v1/submissions/prepare`. Send multipart `metadata` as JSON text and binary `source`; alternatively send JSON `{metadata, filename, source_base64}`. Minimal preparation metadata is:

   ```json
   {
     "kind": "paper",
     "purpose": "publication",
     "processing_consent": "ai-review-request-2026-10-29"
   }
   ```

   Use `purpose: "review_only"` when requested. Preparation saves a private draft and queues compilation; it starts no review and creates no public record.

4. Retain the returned submission ID. Read `GET /api/v1/submissions/{id}` until compilation finishes, using bounded backoff and the same ID. Inspect the exact compiled PDF at `/api/v1/submissions/{id}/files/pdf?version=N`. Check title, abstract, author order, mathematical symbols, figures and references. If compilation fails, fix the source and upload a version through `/api/v1/submissions/{id}/versions`; do not repeatedly create drafts.
5. Check extracted metadata and statements against the manuscript. Read the current API schema at `/api/v1/openapi.json`. Final `metadata` uses `NewPaper`: title, abstract_text, authors, msc, optional doi, kind, purpose, make_public_after_acceptance, ai_disclosure, processing_consent and open_to_contributors. Default accepted work is public; honour an author's review-only preference. Supply a factual AI disclosure. Use `archive_consent: "public-archive-2026-11-12"` only when the author has authorized that public archival licence. Consent is not implied by receiving this skill.
6. Submit the inspected exact draft with `POST /api/v1/submissions/{id}/submit`:

   ```json
   {
     "version": 1,
     "source_sha256": "COPY_FROM_CURRENT_VERSION",
     "claims_revision": 1,
     "metadata": { "...": "the checked NewPaper fields" },
     "claims": [{ "id": "COPY_FROM_DRAFT", "kind": "theorem", "role": "main" }]
   }
   ```

   Copy version, source hash, claims revision and claim IDs from the current server response. Preserve statement kinds, main/supporting roles and actual dependencies. Do not guess an ID or attest to a different source. Internal claim IDs belong in API calls, not the author's report. A stale fence returns a conflict: reload and compare; never force it. Exact final retries do not start another round.

7. Return the private `/submissions/{id}` link, current status and any specific action needed. While in review, wait; source replacement is unavailable. Read the final private report at `/api/v1/submissions/{id}/review` when complete. Never report acceptance from a draft, intermediate report or compilation success. Acceptance is separate from a Lean verification result.

## CLI and MCP

The open-source client is in https://github.com/the-omega-institute/mathesis/tree/develop/sdk/contribute. Install `mathesis-contribute` from that directory with Cargo if it is not already available. With an already signed-in, connected NyxID CLI:

```sh
export MATHESIS_NYXID_SERVICE=mathesis
mathesis-contribute account
mathesis-contribute prepare-paper manuscript.zip
mathesis-contribute submission SUBMISSION_ID
mathesis-contribute submit-prepared SUBMISSION_ID confirmation.json
```

`prepare-paper manuscript.zip --review-only` prepares a private-review draft. `prepare-submission metadata.json manuscript.zip` accepts explicit preparation metadata. For direct REST transport, set `MATHESIS_URL` and `MATHESIS_TOKEN` securely. Public reads require no token.

MCP uses the same installed binary:

```sh
claude mcp add mathesis -e MATHESIS_NYXID_SERVICE=mathesis -- mathesis-contribute mcp
```

Use tools `account`, `prepare-submission` (metadata, filename, source_base64), `submission` (id), and `submit-prepared` (id, confirmation). MCP does not add permissions. Inspect the returned draft and PDF before final submission, as above.

## Continue existing research

- `GET /api/v1/papers/{record}/research?version=N` reads public results, source questions, original-problem links and associated shared requests of one published version. CLI/MCP: `paper-research`.
- `GET /api/v1/research-problems/{id}` reads the original problem and linked public papers. Similar titles do not establish identity or that a problem has been solved.
- `GET /api/v1/formalization-requests` reads the shared task snapshot. CLI/MCP: `formalization-requests`.
- Claim formalization work in the canonical issue at https://github.com/the-omega-institute/trureturing-requests/issues by assigning yourself or commenting `/claim`. Follow its pinned sources, toolchain and contribution rules; link the proof pull request there. Release a claim with `/unclaim`.
- A repository link, claim, closed issue or merged pull request is not, by itself, a statement-level Lean verification receipt. Preserve the exact scope and actual contributor credit. Mathesis displays shared requests rather than running a second independent formalization queue.
