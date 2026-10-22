# chrono-cd worker restart loop, 22 October 2026

Record of a live configuration change to the Omega Research deployment at
<https://mathesis-omega-research.apps.chrono-ai.fun> (chrono-cd project `omega`,
app `mathesis`). It changes no code and no image. It complements
[CMA_DEPLOYMENT_2026-10-22.md](CMA_DEPLOYMENT_2026-10-22.md) (PR #18), which fixes
the credential side.

## Observation

- `GET .../status` reported `degraded`: `web`, `wishpool-api` and `verifier` ready;
  `worker` `ready=false` with **223 restarts** since the previous apply at
  2026-10-21T13:25:34Z (revision 7). Stored logs show the same cycle every five
  minutes: `starting Mathesis`, `/readyz` 503 every ten seconds, `shutdown requested;
  draining`, `stopped`.
- The worker `/readyz` requires the pinned CMA workspace to be `ready` and its
  sandbox `active`. The pinned workspace `wks_8165d1fccfad4f31aa4e12b2d353229a` was
  `preparing` and its sandbox `sbx_532c3e9837e44316bcd5d8886c15e128` was
  `installing` since 2026-10-21T12:45Z. The CMA checks answered in 150–350 ms, so the
  failure was an explicit refusal, not a timeout.
- chrono-cd uses one `health_path` for startup, readiness and liveness probes. With
  `health_path: /readyz` the platform treated execution readiness as process death
  and restarted the worker (repository finding CD-04, platform feedback item 3 in
  the CMA deployment record).
- The first worker revision (2026-10-21T09:56Z, no review model and no Oracle
  configured) had zero restarts, which isolates the cause to the CMA readiness branch.

## Change applied

`PUT api/v1/projects/omega/apps/mathesis?org_id=<Omega Research>` with
`If-Match: 6`, the whole spec unchanged except these five fields:

| Service | Field | Before | After |
|---|---|---|---|
| worker | `health_path` | `/readyz` | `/healthz` |
| wishpool-api, worker | `WISHPOOL_CMA_WORKSPACE` | `wks_8165d1fccfad4f31aa4e12b2d353229a` | `wks_780bf51d0d0d41e99efc34a796b48b40` |
| wishpool-api, worker | `WISHPOOL_CMA_AGENT_PROFILE` | `agp_5119efbbf5fe444db826efcb7b347e54` rev 2 | `agp_a5ca5b123cb51f1719e9e66b387b3d76` rev 1 |

The new workspace lives in sandbox `sbx_04d53d27fe9a481ca8be6fed252f3f1e`
(`active`, `model_access: chronoai_managed`, workspace `ready` since
2026-10-22T07:54:43Z). Both identities are the ones the CMA deployment record names
as its verified resources. Images stay `ghcr.io/chronoaiproject/mathesis-*:1de3015`.
Result: generation 7, revision 8, reconcile ok at 2026-10-22T08:08:26Z.

`/healthz` only states that the process serves. Execution readiness (TeX, CMA
resources, verifier) is still reported by the worker's `/readyz`, which is internal
to the namespace; read it from a sibling service or through the worker logs when
a review is admitted. This change removes the restart loop; it does not by itself
make reviews run.

## Verification

Status polled through `GET .../status` after the apply:

| Time (UTC) | Phase | Worker pod |
|---|---|---|
| 08:08:38 | progressing | old pod `cdrld`, restarts 224; new API pods rolling |
| 08:09:40 | ready | `worker-c45d7c4f-64k75` ready, restarts 0 |
| 08:16:28 | ready | same pod, restarts 0, started 08:09:04 |

The new worker ran past the previous five-minute kill point without a restart.
Its log shows `starting Mathesis` and four `paper worker started` lines and no
`shutdown requested`. `web` and `verifier` pods were not rolled. No review was
submitted; CMA execution with the current credential was not exercised.

## Still open before reviews run

1. The mounted CMA credential at `/run/secrets/cma-token` is an expired user access
   token. The running image refuses `nyxid_ag_` keys, so the durable dedicated key
   needs the patched adapter from PR #18.
2. That image is being published as `ghcr.io/the-omega-institute/mathesis-api:cma-ae641a0`
   by the one-off workflow on `deploy/cma-api-20261022`. Packages under
   the-omega-institute are private; chrono-cd needs either a classic GitHub PAT with
   `read:packages` sent once as `registry_credential`, or the package set to public.
3. After the image swap: set the dedicated key through the secret-files endpoint,
   then confirm the worker's execution readiness and run one complete manuscript
   review as the CMA deployment record requires.

## Rollback

Send the previous spec (revision 7 values above) with the current `If-Match`.
The restart loop returns with it; nothing else changes.
