# CMA deployment verification, 2026-10-22

Mathesis is deployed independently in Omega Research at
https://mathesis-omega-research.apps.chrono-ai.fun. The ChronoAI app is separate.

## Verified integration differences

- Read-only inspection of CMA develop revision
  `40cc99c9bbccfbb5a9a71d36053303658370d1ae` found service-bound API-key validation in
  `api/crates/cma/src/auth/agent_keys.rs`. Mathesis incorrectly refused every
  `nyxid_ag_` credential before sending any request.
- A dedicated nonexpiring NyxID key limited to the operator's CMA service returned
  HTTP 200 through `https://nyx-api.chrono-ai.fun/api/v1/proxy/s/cma`.
  The same credential returned HTTP 401 at both direct CMA origins. The NyxID
  catalogue still claims agent keys also fail through the proxy; that statement
  does not match this live test. No upstream configuration was changed.
- The old Sandbox `sbx_532c3e9837e44316bcd5d8886c15e128` remains installing;
  Workspace `wks_8165d1fccfad4f31aa4e12b2d353229a` remains preparing. Its old profile
  initializes the trureturing repository. Repeated `create_profile_workspace`
  failures are summarized as `lifecycle dependency unavailable`. The public API
  suppresses the underlying transient error; this does not prove all CMA is down.
- The documented Agent Manager `POST /api/v1/agents` flow created an empty
  Mathesis environment successfully in 12 seconds (07:54:38–07:54:50 UTC).
  The creation used one stable idempotency key and retained its original receipt.
- A real model response through the dedicated service key completed with
  `MATHESIS_CMA_OK 323`; response `resp_a9340e3e637c41d3b4e73f7ef6e4f517`.
  This verifies model execution, not manuscript review or acceptance policy.

The patched Mathesis adapter also passed a live preflight and a complete durable
Agent request, returning `{"status":"ok","product":323}`. It retained response
`resp_ad3e647ad2884b948dfd76dc91bdda06` and Agent
`agt_3784c22e83c74cbb83b0217ac13e2f7d` for asynchronous cleanup; its subsequent public state is `deleted` and the
worktree is `removed`. Local validation covered 384 workspace tests, including
MongoDB and TeX integration, and 331 frontend tests. The independent smoke does
not prove that a manuscript can complete the full publication pipeline.

## Resource identity

- Lifecycle operation: `450ba832963444248ad648c66dfdb27d`
- Sandbox: `sbx_04d53d27fe9a481ca8be6fed252f3f1e`
- Workspace: `wks_780bf51d0d0d41e99efc34a796b48b40`
- Agent profile: `agp_a5ca5b123cb51f1719e9e66b387b3d76`, revision 1
- Setup/smoke Agent: `agt_2ebe77cd820846029e2d2d2b4e4ec382`

Use the mounted dedicated key with the proxy URL. Never copy a local user access
token into an unattended deployment without a refresh mechanism. The earlier
mounted user token expired; the local CLI refreshing does not update the mounted
file. Credentials are excluded from this record and from Git.

## Platform feedback

1. Update the CMA service catalogue's key limitation after checking the proxy
   identity contract; direct and proxied authentication currently differ.
2. The old profile initialization retries indefinitely without exposing a useful
   failure cause. Revision increments count retries, not completed work.
3. Chrono-CD `internal/render/workload.go` uses `health_path` for startup,
   readiness and liveness probes. Using Mathesis `/readyz` for worker liveness
   repeatedly restarts the container on an external CMA outage. Process liveness
   is `/healthz`; execution readiness must still be checked independently.

## Validation incident and security patches

The required API job could not start its MongoDB 8.0 service on the GitHub
runner's Linux kernel (6.19 or newer). Its container reports SERVER-121912 and
exits before application tests. The same workspace test command is validated
against a disposable, loopback-only local MongoDB with the TeX test binary set;
the existing shared CI workflow is unchanged.

The existing security checks also identified outdated dependencies. Update Vite
to 7.3.7, rustls-webpki to 0.103.16 and webpki-roots to 1.0.10 within their existing
version constraints. The Rust patches address RUSTSEC-2026-0382 and
RUSTSEC-2026-0381. Do not deploy the earlier image built before these patches.

The proxy occasionally returned HTTP 500 or `503 store_failed` during follow-up
reads, with `Authentication authority is temporarily unavailable`. Subsequent
reads succeeded. Successful smoke tests establish capability, not uninterrupted
platform availability.

## Remaining launch gates

Deploy the patched API/worker image and dedicated key, switch to the ready
workspace, then verify a full manuscript upload, PDF build, review, decision and
final report. Resolve publication knowledge-index requirements and test
formalization separately. BYOK remains disabled until explicitly configured.
