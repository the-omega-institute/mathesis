# Mathesis chrono-cd beta integration, 20 October 2026

This is a record of an actual integration exercise, including unresolved platform
questions. It is not a claim that the public application or AI review service is
operational. The operator authorized preparing and publishing the container
images and recording unclear steps.

## Source and execution scope

- Repository: <https://github.com/the-omega-institute/mathesis>.
- Isolated branch: `codex/chrono-cd-beta-20261020`.
- Base revision: `1de3015ce0268f9a42e7e777aea0211b89a434b3`.
- The source snapshot includes the current author/staff interface simplification
  and English UI. The original working directory remains available with its
  uncommitted changes.
- The temporary `chrono-cd-beta-images.yml` workflow runs only for source or
  workflow changes on this branch. It is not intended for merging as a permanent
  release workflow. Existing CI, production workflows, branch policy and upstream
  services are unchanged.
- The workflow repeats the repository's structure, API, SDK, frontend and secret
  checks before publishing. The verifier build runs the existing real Lean tests.
- Builds run on ephemeral GitHub-hosted Linux amd64 runners. There is no Docker
  executable on the local Mac. No local Lean compilation is started.
- Images use unique beta tags, source/revision OCI labels and recorded registry
  digests. No `latest` or existing release tag is overwritten.

## Platform and account observations

- Skill: <https://ornn.chrono-ai.fun/skills/chrono-cd>, version **1.2**.
- Read the bundled `SKILL.md`, `references/api.md` and `references/errors.md`.
- The skill page is a web application. Its machine-readable package was fetched
  through the existing login using
  `nyxid proxy request ornn-api '/api/v1/skills/chrono-cd/json?version=1.2' --output json`.
- `nyxid proxy request chrono-cd api/v1/account --output json` succeeds.
- The current account is an admin of **Omega Research**. Organization-scoped
  project and quota-request lists were empty at the start of this exercise.
- `/account` describes personal quota (2 CPU, 4 GiB requested memory, 10 pods,
  20 GiB PVC storage). It does not establish the organization's actual quota.
- The documentation's four-service example explicitly uses Mathesis/Wishpool.
  Example tags are not evidence that packages exist or are accessible.

## Findings and open questions

| ID | Observation | Impact and current handling |
|---|---|---|
| CD-01 | NyxID **0.7.1** returns `unrecognized subcommand 'connect'` for the documented `nyxid connect github --scope read:packages` flow. `nyxid provider --help` only lists disconnect. | The documented private-registry connection path is unavailable in this installed CLI. Record the required supported CLI version or actual supported connection flow; do not ask for credentials in chat. |
| CD-02 | `SKILL.md` describes a classic PAT as the private-image mechanism, while `references/api.md` also documents an optional NyxID GitHub connection. | Clarify which mechanism the live deployment enables, the expected scopes, and how an agent verifies the connection without exposing its token. |
| CD-03 | The current `gh` OAuth login has repository/workflow access but no `read:packages` or `write:packages`. Package listing returns HTTP 403. Anonymous access to the example `wishpool-*:0.2.0` tags also failed. | These responses do not establish whether the images are missing or private. Publishing uses the repository's short-lived Actions `GITHUB_TOKEN` with job-scoped `packages: write`; no personal token is added. Registry pull authorization for chrono-cd remains a separate check. |
| CD-04 | chrono-cd exposes one `health_path` for startup, readiness, liveness and edge health checks. Mathesis separates `/healthz` from `/readyz`; worker readiness checks MongoDB, CMA and the verifier and can take several seconds. | Blindly using `/readyz` as liveness can restart healthy processes during a dependency outage. Confirm probe timeouts and supported separation. A process-only probe needs separate readiness monitoring and must not be presented as proof that reviews execute. |
| CD-05 | The original worker uses one PVC owner, `Recreate`, and a 65-second termination grace period. These update/drain settings are not fields in the documented chrono-cd spec. | Confirm PVC update semantics and shutdown grace before using the platform for live review jobs. Mathesis retains remote job identities and leases, but that does not establish the platform's behavior. |
| CD-06 | Organization quotas are not returned by the documented personal `/account` response; the spec charges update surge resources. | Establish the organization quota and surge calculation before applying all four services. Do not reduce Lean resources simply to fit an assumed quota. |
| CD-07 | The frontend accepts uploads through nginx with a 64 MiB proxy limit and 300-second read timeout. chrono-cd can route `/api` and `/auth` directly to the API, but edge upload/time limits are not documented. | Test source and Lean uploads, callback handling and forwarded-origin behavior on the chosen routing path. Preserve same-origin login and CSRF enforcement. |
| CD-08 | The platform provides a managed HTTPS hostname; no per-app custom-domain endpoint is documented. | Use the managed hostname for beta. Confirm custom-domain support separately; do not change global admin domain settings. |
| CD-09 | App `PUT` is a full replacement and is not a validation endpoint. A `422 missing_secret_keys` can still store the spec and deploy other services. | Read existing state before updates, use generation fencing, and do not use deployment mutations for speculative validation. A dry-run/validate endpoint would improve this flow. |
| CD-10 | The MongoDB add-on documents provisioning and rotation, but the skill does not establish backup/restore policy or retention. Image rollback cannot reverse Mathesis's stored review formats. | Confirm backup/restore before production; retain readers compatible with review contract 3 and the GridFS snapshot format. See `docs/OPERATIONS.md`. |
| CD-11 | The sample spec omits several application-required values, including the public URL, verifier workspace/bind settings and live review configuration. | Treat it as a platform syntax example. Derive the runnable app spec from Mathesis configuration and runtime checks. |

## Application prerequisites distinct from container publishing

- A real NyxID confidential client and callback for the chosen HTTPS hostname.
- MongoDB connection and a persistent encryption key when storing delegated tokens.
- An accessible Oracle pool and CMA operator workspace with an exact published
  agent-profile revision; the knowledge source must also be independently admitted
  before new publication reviews can satisfy their evidence gate.
- An explicit BYOK rollout choice. `WISHPOOL_CMA_BYOK_ENABLED` defaults to false;
  publishing images does not turn it on or provision author workspaces.
- Runtime verification of login, upload/PDF compilation, a completed review and
  publication decision, attribution, and formalization states.

## Publication results

Pending the isolated build run. Record its URL, exact source revision, image
tags/digests, package visibility and actual smoke-test outcomes here when complete.

No platform feedback message has been sent; this document is the reviewable
record for later sharing.
