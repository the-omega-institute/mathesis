# §11. Referee rounds and feedback

Part of the repository contract in [CLAUDE.md](../../CLAUDE.md); equally binding.

- S1 confirmation queues a referee round on the exact paper version and
  statement revision. Layer 2 owns ordering, input fencing, immutable settled
  steps, report filing, decision application, letter validation and view rules.
- The publication pipeline is GPT Pro referee → Codex audit → automatic escape-analysis
  decision → advice → automatically delivered letter → accepted-only Lean probe.
  Publication is immediate when the selected visibility is public. Paper/note
  probes and conjecture statement elaboration follow the delivered letter.
  No step requires a human editor. Editors/admins may intervene, restart settled
  reviews while the paper is in review, or send additional letters. Staff can also
  restart a settled, failed current private review after review completion. This
  records a new operational re-review round without changing the source, prior
  readings/letters, visibility or publication state; successful private reviews
  and live rounds cannot use this recovery path.
- For conjectures, the referee supplies well-posedness reasons, open/known/unclear
  status, reported work names, content/bind-only escape reasons and sharpening
  suggestions. CMA Codex checks missing symbols/quantifiers and small cases, and
  may search literature and open sources. Known-result matches require a
  matching argument and the reported opened source, or reproducible local
  mathematical evidence. Unopened external knowledge becomes unclear.
- The audit checks source, report JSON and full text, computing where useful.
  It covers every confirmed statement with correctness, a one-sentence comment,
  proof shape/witnesses where applicable, and agreement with the referee.
  External facts the agent could not open are `not_checkable`. Every named
  prior work includes its reported opened source; never invent citations.
- Layer 2 appends audit-derived S2 (known prior works) and S3 (main-result escape
  assessments) under the statement revision fence. Gap/error/not-checked main
  results have no escape witnesses and cannot ground acceptance. Publication is
  decided only by `Policy::decide`, never either model's recommendation.
  Report and decision replay is idempotent, including recovery after record
  insertion. Accepted records and their publication inputs remain immutable.
- Advice runs after the applied decision (or private review completion) for every audited paper and feeds
  improvement suggestions to the letter. Formalization candidates are used only
  after acceptance. A sketch does not verify a theorem.
- A completed letter step atomically delivers the letter in-app, with its round,
  applied accept/reject outcome (no assessment for private or pending reviews),
  the auditor as sender, and `edited=false`. It states the
  applied decision and record/reasons first, then audited feedback, confirmed
  changes, "please check" for unavailable facts, and improvement suggestions.
  Author-facing reports do not use minor/major-revision outcome labels; accepted
  reports describe contributions and extensions, refusals explain their reasons.
  Internal mathematical recommendations and historical letters stay unchanged.
  New rounds pin `letter_contract: 3`; legacy rounds retain their original prompts.
  The My work list uses the private, version-bound `review-summary` projection;
  delivery requires a saved current letter. Internal stages remain in staff details.
  It contains no Lean results. Private review letters state completion and feedback,
  never acceptance/rejection. Manual additional letters remain available.
- Lean runs only after a delivered letter and acceptance, using advice's proved
  candidates. The binary compiles each file and checks axioms. Authors see
  statement, outcome and theorem name afterwards; no second letter is needed.
  A compiled probe is not a recorded formalization; publishing files needs the
  author's agreement.
- Author-safe rounds include payload-free stage progress: state, recorded times and
  retryability, including hidden advice and letter stages. Results, free-form errors
  and provider metadata never enter this progress projection. Only an actual delivered
  letter establishes delivery; queued author replies remain visibly awaiting re-review.
  A persisted temporary service failure adds `waiting_since` to the current round
  and review summary. Successful provider communication clears it. It preserves
  the original request identity and deadline; it is never inferred from elapsed time.
- Staff receive full review history. Submitter and linked co-authors receive
  reports, audits, sent letters and projected Lean outcomes through the existing
  referee GET; advice, drafts, provider metadata and Lean files remain private.
  Anyone else receives `not_found`, including readers of an accepted paper.
- Referee readings remain machine judgements under `wishpool:referee` by default
  (engine `nyxid-oracle`). Audit reports and automatic letters use the separate
  `wishpool:auditor` account by default (engine `nyxid-cma`; `codex-cli` only for explicit development fallback). All model output is
  untrusted and sanitised in the binary; never invent witnesses or citations.
- A referee request goes to every configured Oracle pool; the first completed
  answer is the report. The default pool is `chrono-chatgpt-pro-pool` on the
  standalone Oracle broker, reached through NyxID's proxy service `oracle`
  (`nyxid proxy request oracle api/v1/oracle/...` for the local CLI).
- The NyxID Oracle task and submission identity are durable; the worker releases
  its lease between polls without spending retries and renews a running job's
  30-minute lease about every five minutes. A lost lease fences the pending
  result. Transport completion is not mathematical verification. Model output
  is sanitised in the binary.
- All Codex agent steps use CMA through NyxID: audit, advice, letter,
  accepted-paper Lean probe, conjecture target generation and paper-problem
  audits. CMA network access is on. Paper TeX and needed inputs travel inline,
  split into turns above 200 KiB. A final JSON object split across assistant
  messages in one response is assembled from exact ordered bytes, within the
  existing answer bound; earlier commentary is excluded and saved terminal
  answers are not rewritten. Final messages are JSON; Lean entries carry
  every file's complete text. Answers are capped at 2,000,000 bytes.
- CMA agent/response ids, chunk progress, deadlines and terminal answers are
  durable private steps in the referee aggregate. Stable keys bind submission,
  version, statement revision, round, step and attempt. Worker restarts resume
  polling with those ids; uncertain mutations reuse the original key and body.
  Agent steps have at most 3,600 seconds. The lease heartbeat stays active.
  New CMA runs pin delivery contract 1: the final input chunk repeats bounded
  audit/advice/letter instructions and the exact response-format requirement without removing
  input bytes. Legacy saved runs retain their original framing and keys. Only a
  terminal `agent_command_queue_timeout` with explicitly empty output may retry
  its chunk once on the same agent, after saving the original response receipt,
  with a distinct durable key and within the original deadline. Live work,
  ambiguous failures and executed output are never restarted by this path.
- A worker has one compile lane, one verification lane and 1–8 remote review
  lanes (`WISHPOOL_REVIEW_CONCURRENCY`, default 2). Compile jobs never wait on
  CMA cleanup. Each job retains its own lease and cancellation fence.
- Every CMA configuration pins a published agent profile. Before a new Oracle
  request is admitted, read-only CMA resource preflight checks the eligible
  author/operator resources without creating an agent or recording a payer.
  Execution admission still chooses and persists the payer in Layer 2; existing
  attempts never change identity during preflight. Resource readiness is not
  proof of working model credentials or quota.
- CMA output is untrusted: the binary writes returned Lean text into fresh
  scratch and checks it locally; the credential-free solution verifier is
  unchanged. Always stop and delete the CMA Agent after the final answer or on
  cancellation/failure, best effort with logged failures. Credentials stay in
  the transport, never in a model prompt.
- `WISHPOOL_AGENT_BACKEND=cma|codex-local` selects the carrier; configuring a
  CMA workspace defaults to CMA. `codex-local` is an explicit development
  fallback, refused on non-loopback binds. Oracle CLI and CMA CLI are also
  loopback-only. CLI children inherit only operator `PATH` and `HOME`.
  The local fallback uses fresh source/scratch copies and disables its sandbox
  network. Production CMA HTTP through NyxID accepts an operator API key scoped
  to CMA, or a user access token from the existing sealed refresh path or a mounted
  token file. NyxID/CMA enforce current authority; credential prefixes are not
  authorization. Mounted user access tokens do not refresh automatically.
  Every new configuration key must be declared in infra (§8).

- Author CMA connections use ordinary NyxID `offline_access` consent, without
  resource or service restrictions, in separate sealed `cma_review` generation
  slots. `WISHPOOL_CMA_BYOK_ENABLED` defaults false; enabling it requires NyxID,
  HTTP CMA transport, `WISHPOOL_TOKEN_KEY` and `WISHPOOL_CMA_PUBLIC_URL`.
  Authors choose an existing ready workspace owned by their NyxID subject in
  a Sandbox whose pinned profile uses `own_credentials`. Automatic provisioning
  is a follow-up. Connecting does not require hosted donations.
- Before any CMA I/O, Layer 2 durably admits the submitting author's ready
  binding for eligible submission steps or the operator default. Co-authors
  are never selected as payers; supporter attribution is separate from account custody. Replays retain the attempt's payer, generation and workspace.
  A terminal author account/auth/setup/credential/quota refusal, or an agent
  failure labelled `cause_unknown`, admits at most one recorded operator
  successor with fresh request keys. Transport errors, 429/5xx, uncertain writes,
  mathematical/JSON errors and local verifier failures never switch payer.
  Cleanup uses each attempt's original account. Private referee views expose
  only the step, ordinal, payer, sanitized reason, status and times, before letters.

Author CMA execution requires `WISHPOOL_CMA_AGENT_PROFILE` with an exact
`{"id":"agp_…","revision":1}` pin to a published agent profile readable by
both the operator and author accounts. The exact pin is retained in each
creation intent for retries. Workspace facts are fetched and decoded in Layer 3;
Layer 2 checks ownership, readiness and `own_credentials` before saving the
binding under its source revision. Refresh throttling, outages, ambiguous
refusals and concurrent token rotation remain transient, with the same payer.
Disconnect/reconnect revokes the old generation; outstanding agent cleanup is
best effort and may fail after revocation. Cleanup never substitutes operator
credentials for author resources.

CMA payer mapping (Layer 2 decides before provider I/O):

| Step | Payer and attribution |
|---|---|
| Referee audit, advice, letter, accepted-paper Lean probe | Ready submitter binding; otherwise operator |
| Standalone conjecture target, admitted paper-problem target, author-approved proved paper-statement target | Ready submitter binding; otherwise operator |
| Target faithfulness audit, including regeneration | Ready submitter binding; otherwise operator |
| Independent paper open-problem audit | Ready submitter binding; otherwise operator |
| Error-report text audit | Operator, never the author or reporter |
| Venue-agent attempt on a confirmed target | Operator account with phase-D donation attribution, never the author |
| Existing legacy CMA run | Original operator account; never retroactively charged to an author |

Operator-funded target generation, faithfulness audits, Lean probes and venue
attempts retain phase-D supporter attribution, including an operator successor.
Author-funded steps create no donation attribution. Attribution alone charges no
tokens: only a verified NyxID usage receipt can meter usage. Replays retain the
saved payer, workspace, generation and request identity. Oracle remains operator
funded; local elaboration and proof verification carry no CMA payer.
