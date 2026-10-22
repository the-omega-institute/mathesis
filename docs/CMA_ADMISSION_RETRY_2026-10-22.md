# CMA turn admission conflict, 22 October 2026

Record of the first automatic review on the Omega Research chrono-cd deployment
and the adapter change it required. It extends
[CMA_DEPLOYMENT_2026-10-22.md](CMA_DEPLOYMENT_2026-10-22.md); identifiers only,
no credentials.

## Observation

Review-only submission `01a16476-e76b-7108-9581-2d369941c276` (one trivial
theorem) was uploaded at 16:34Z against the image built from `406096d` with the
dedicated CMA key mounted.

| Step | Result |
|---|---|
| Source read, PDF compiled, consent recorded | pass |
| Knowledge queries (contract 3, three admitted and completed) | pass |
| GPT Pro referee (`gpt_6 · pro`) | done 16:42:27Z, seven minutes |
| CMA audit attempt 1 (`r1:audit`, agent `agt_f5acf74c…`) | succeeded 16:45:41Z–16:49:41Z |
| CMA audit attempt 2 (`r1:knowledge-followup:audit`, agent `agt_9a79399b…`) | failed 16:51:50Z after six seconds |
| Advice, letter | skipped: no completed audit |

The second attempt's recorded reason is
`unusable model output: provider returned 409: agent_command_admission_conflict`,
`retryable: false`. The agent was created at 16:51:45Z and deleted at
16:51:50Z; CMA shows no first-message preview for it, so the refused turn was
its first.

## Cause

`layer3-review::cma::Client::advance` posts the first turn one poll interval
(five seconds in production) after the creation receipt. A fresh agent is
still `starting` while its `first_message` runs. A direct probe on the same
workspace and profile at 17:13Z reproduced the sequence: creation receipt with
`status: starting`; a turn posted one second later answered

```
409 {"error":{"code":"agent_command_admission_conflict",
 "message":"The Agent cannot admit the request from this observation. Retry using the same key."}}
```

and the identical turn posted four seconds after that was admitted
(`response.created`). The probe agent was stopped and deleted afterwards.

The adapter classified every 409 as a terminal refusal: `transient` covered
only transport errors, 429 and 5xx, so the run loop returned the error, the
step was recorded as failed and not retryable, and the review stopped before
advice and letter.

## Change

`transient` now also covers `409 agent_command_admission_conflict`. The run
loop then sleeps one poll interval and replays the same turn with its durable
key `<step>:turn:<n>`, exactly as the provider asks; the step deadline still
bounds the retries. `failure_kind` is unchanged, so the refusal never selects a
payer successor. A unit test covers two consecutive conflicts followed by
admission, same key and body on every post, one agent creation, normal cleanup.

Earlier rounds that recorded this failure are not rewritten. Restarting the
settled review through the existing staff route runs the audit again under the
same round inputs.
