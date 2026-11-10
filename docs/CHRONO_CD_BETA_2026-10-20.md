# Mathesis chrono-cd beta integration, 20 October 2026

This is a record of an actual integration exercise, including unresolved platform
questions. It is not a claim that the public application or AI review service is
operational. The operator authorized preparing and publishing the container
images and recording unclear steps.

**Result:** all four Linux amd64 images are published as private packages linked
to the Mathesis repository. Source checks, container smoke checks, and two real
Lean verifier tests passed. chrono-cd still rejects the private image with
`image_unauthorized` (13003), including when retested with NyxID 0.71.0. No live app
was created. Exact machine-readable receipts are in
[CHRONO_CD_IMAGES_2026-10-20.json](CHRONO_CD_IMAGES_2026-10-20.json).

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
| CD-01 | Initially installed NyxID **0.7.1** lacks `connect`. A temporary, checksum-verified **0.71.0** release supports it. | Resolved as an old-client mismatch, not a current CLI defect. The skill should state a tested CLI version and suggest `nyxid update --check` before credential setup. |
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
| CD-12 | The current NyxID account already has active `api-github` and `api-github-pat` connections. Reading organization packages through the OAuth connection returns HTTP 403 because **the-omega-institute restricts OAuth app access**. The PAT connection returns HTTP 403 because it lacks `read:packages`. | A connected GitHub account is not sufficient evidence of usable organization package access. Separate connection state, package scopes and organization authorization in the setup instructions. This exercise does not relax the organization's access policy. |
| CD-13 | NyxID **0.7.1** exits **0** after denied package requests and mixes a text prefix with JSON. Retesting the same denied PAT package read with **0.71.0** returns **1** and a structured JSON error on stderr. | Fixed in the current tested release; do not report it as a current platform defect. Scripts using older clients need to check the actual error response. |
| CD-14 | An actual organization-scoped zero-replica apply using the published web digest returned HTTP **400**, `image_unauthorized` (**13003**), with `private package: supply registry_credential`. | Private-package pull is a reproduced deployment blocker, not just an inference from GitHub API permissions. The live message only suggests the explicit credential path. The documented automatic GitHub-connection mechanism was not demonstrated by this request. |
| CD-15 | Mathesis's Kubernetes manifests set `fsGroup: 65532` so non-root API/worker/verifier processes can write their mounted cache and scratch directories. chrono-cd documents volumes and non-root execution but no volume ownership or `fsGroup` control. | Confirm how the platform makes PVC/emptyDir mounts writable for image UID 65532 (and frontend UID 101). Standalone image smoke tests do not prove mounted-volume permissions in the platform. |
| CD-16 | With **0.71.0**, the documented `connect github --scope read:packages --no-wait` request fails at `/connect-links`: HTTP **403**, `insufficient_scope`, `write or admin scope required for this operation`. | A working proxy login and organization admin role do not necessarily authorize creating connection links. The service's guidance says to have the owner review the required scopes and not switch identity or widen permissions automatically. No connection link or wider authorization was created. |
| CD-17 | The live catalog returns **404** for `github`; `api-github` exists and advertises `read:packages` support. | Confirm whether the connect-link API supports a `github` alias or update the skill to the live catalog slug. The scope rejection in CD-16 happens before this could be established through a successful connection request. |

### CLI version recheck

`nyxid update --check` reported installed v0.7.1 and latest **v0.71.0**. The release
is <https://github.com/ChronoAIProject/NyxID/releases/tag/v0.71.0>, published
15 October 2026. Downloaded the macOS arm64 archive into a temporary directory,
verified its release checksum, and successfully ran `gh attestation verify` for
the repository. The default CLI installation was not replaced.

The temporary CLI at
`/tmp/mathesis-release-tools/nyxid-v0.71.0/nyxid` confirmed that CD-01 and CD-13 are
resolved in that release. A new-version chrono-cd account read returned the same
subject and the same Omega Research admin role as the original account check.
Credential scopes, organization package access, and chrono-cd pull authentication
remain separate unresolved requirements. No token value or hosted connection URL
is included in this report.

Using the existing deployment resource requests, steady state is 1,490 millicores,
2,880 MiB and six pods. If one surge pod is charged for each of four services,
the reservation becomes 2,860 millicores, 5,600 MiB and ten pods. If the PVC worker
has no surge, it is 2,610 millicores, 5,088 MiB and nine pods. Both interpretations
exceed the documented default CPU/memory quota. The actual organization quota
and PVC strategy must determine the final request; these calculations are not a
live quota rejection.

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

The existing Dockerfiles need these deployment-specific inputs; they are not
baked into the images:

| Component | Required deployment configuration |
|---|---|
| Web | Port 8080. The bundled nginx proxy resolves `wishpool-api:8080`; preserve that internal service name or supply an explicitly reviewed nginx configuration. |
| API | `WISHPOOL_ROLE=api`, `WISHPOOL_BIND=0.0.0.0:8080`, real `WISHPOOL_PUBLIC_URL`, `WISHPOOL_STORAGE=mongo`, MongoDB add-on environment, `WISHPOOL_AUTH_MODE=nyxid`, confidential-client secrets and initial administrator subjects. |
| Worker | Same application revision/database/authentication configuration as API; `WISHPOOL_ROLE=worker`, TeX cache at `/var/cache/texmf`, Oracle HTTP token/configuration, CMA workspace and published profile pin, and an actual admitted knowledge-index revision. |
| Verifier | Port 8081, `WISHPOOL_VERIFIER_BIND=0.0.0.0:8081`, `WISHPOOL_LEAN_WORKSPACE=/opt/lean-workspace`, `TMPDIR=/scratch`, writable scratch volume, no secrets and no external egress. |
| API/worker verifier client | `WISHPOOL_VERIFIER_URL` must use the actual internal verifier service name and port. |
| CMA HTTP credential | Mount the operator token at the configured `WISHPOOL_CMA_TOKEN_FILE` through the **secret-files** API, outside ordinary volume mounts. Provide it before releasing the worker's missing-secret gate. A short-lived local login token is not a durable production credential strategy. |
| Optional author BYOK | Enable explicitly with a persistent `WISHPOOL_TOKEN_KEY`, CMA HTTP transport, readable published agent profile and `WISHPOOL_CMA_PUBLIC_URL`; test both author execution and the recorded operator fallback. |

Configure `/` for the frontend and `/api`, `/auth` for the API if using direct
platform routing. First resolve CD-04, CD-05 and CD-07 for the selected runtime
configuration. Set credentials via the secret endpoints, never plain `env` or
committed JSON. A final executable deployment spec is intentionally not fabricated
from placeholder credentials, unverified image references or an assumed hostname.

## Publication results

Build run: <https://github.com/the-omega-institute/mathesis/actions/runs/39439640801>.
Source revision: `3e5441ad9c9971154be0a289aec1251987872955`.

The original run retains an overall failure because its first web smoke check
failed; its API and Lean/verifier publication jobs succeeded. The separate web
recovery run succeeded against the same pinned source. The combined publication
is complete; the original failed check remains visible rather than being erased.

All four images use tag **`beta-20261020-3e5441ad9c99`**:

| Image under `ghcr.io/the-omega-institute/` | Purpose | Registry digest |
|---|---|---|
| [mathesis-web](https://github.com/orgs/the-omega-institute/packages/container/package/mathesis-web) | English frontend | `sha256:9ae76863f616ad6062bec221d664452584636599c3426e2eca23c1d9ab1610e9` |
| [mathesis-api](https://github.com/orgs/the-omega-institute/packages/container/package/mathesis-api) | API and worker, selected by `WISHPOOL_ROLE` | `sha256:040f806d6337470fe8cae2b353bb9f2d32b3e7a57446d2d919fb3b791ff62df1` |
| [mathesis-lean](https://github.com/orgs/the-omega-institute/packages/container/package/mathesis-lean) | Prepared Lean/Mathlib base | `sha256:92b3f82e7065c1d2973b2264cc7d96d8824fef28aaa6da8d4d9c7ee369bac20e` |
| [mathesis-verifier](https://github.com/orgs/the-omega-institute/packages/container/package/mathesis-verifier) | Independent Lean verifier built from that exact base digest | `sha256:910af55e98c983a11565daf00ab1f74f5ab0e47f74633995c4e2419b8341df91` |

Validation outcomes:

- `mathesis-web`: published and linked to this repository as a private package;
  non-root UID 101; `/`, author deep link and staff deep link return the English
  application. The recovery run succeeded, including manifest validation.
- `mathesis-api`: published and linked to this repository as a private package;
  API and worker `--check` both report ready under isolated development settings;
  the container actually compiled a one-page PDF (14,120 bytes). These local
  runtime checks use no provider credentials and do not establish AI execution.
- `mathesis-lean` and `mathesis-verifier`: both published and linked to this
  repository as private packages, running as UID/GID 65532. The base pins Lean
  **4.33.0** and Mathlib `db584cd6d46c92f209a44c0f1c829460d327499d`.
- The verifier build ran `real_lean_multimodule_proof_checks_helpers_and_wrong_type`
  and `real_lean_target_proof_disproof_wrong_type_and_axiom`: **2 passed, 0 failed,
  0 ignored**. They exercise real proof/disproof checking, wrong-type and
  forbidden-axiom rejection, and supporting modules. Its final container returned
  successful `/readyz` with `--network none` and its default non-root identity.
- The image receipt JSON records exact digests and uncompressed local image sizes;
  API is 2,732,048,171 bytes because the runtime includes the supported TeX packages.
  The prepared base is 11,275,166,121 bytes and verifier is 11,277,244,465 bytes;
  they share base layers. These figures are not compressed transfer sizes or
  memory requirements. Measure first-pull time and node disk use on the platform.
- Compared all 160 frontend source paths with the original working directory:
  the published snapshot matches the current local frontend.

All source gates passed on this revision: repository structure/layer rules,
API format/clippy/tests/dependency policy, SDK checks, frontend checks and secret
scanning. Frontend evidence: **46 test files, 359 passing tests**, both app and
showcase builds, and zero reported npm vulnerabilities.

### Local publication incident PUB-01: frontend startup race

The first web image built successfully and ran as a non-root user. Its immediate
HTTP smoke check failed with curl exit 56, `Recv failure: Connection reset by
peer`. Logs show the first request at `14:33:01.629 UTC` and nginx startup notices
at about `14:33:01.642 UTC`. The script retried connection refusal but not reset.
No web tag had been pushed at that point.

The existing run could not rerun that job while the other jobs were active
(`job 118687589575 cannot be rerun`). A separate branch-scoped recovery workflow
adds bounded `--retry-all-errors` startup handling, explicitly checks the original
revision's successful source gates, and checks out that **same source revision**.
It leaves the ongoing API and Lean build handles intact. This is a publication
test-harness fix, not a chrono-cd or application failure.

Recovery run:
<https://github.com/the-omega-institute/mathesis/actions/runs/39440497289>.
Workflow revision: `87ebec94511f7a1e18974dd1d0fc36a7eca0439d`;
image source revision remains `3e5441ad9c9971154be0a289aec1251987872955`.

### Live chrono-cd private-registry check

After web publication, sent one app spec to the Omega Research scope with a
single `expose: none`, `replicas: 0` service and the exact published image digest:

```text
ghcr.io/the-omega-institute/mathesis-web@sha256:9ae76863f616ad6062bec221d664452584636599c3426e2eca23c1d9ab1610e9
```

The response was HTTP 400 / `image_unauthorized` / error code 13003:
`private package: supply registry_credential`. The follow-up project list remained
empty and reading the requested app returned HTTP 404 / `app_not_found` / error
code 13000, again with CLI exit status 0. No app, public route, database or running
service was created by this failed check.

After checking NyxID **0.71.0**, repeated that same zero-replica check with the
new release: HTTP **400**, `image_unauthorized` (**13003**), and process exit **1**.
The organization project list remained empty. The current-version result
confirms that updating the CLI alone does not fix package pull authorization.

To continue, chrono-cd needs a supported pull credential authorized for these
organization packages (classic PAT with `read:packages`, using the documented
secret transport), or the package owner must choose public image visibility.
Changing organization OAuth policy or making a private package public is not
silently included in this publishing exercise. Do not place tokens in this report.

No platform feedback message has been sent; this document is the reviewable
record for later sharing.
