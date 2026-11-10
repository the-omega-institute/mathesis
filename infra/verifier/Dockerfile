# syntax=docker/dockerfile:1
# LEAN_IMAGE must be a digest-pinned, prepared image containing the exact
# /opt/lean-workspace and /opt/verifier/.elan/toolchains named by its manifest.
ARG LEAN_IMAGE
FROM ${LEAN_IMAGE} AS lean
FROM rust:1.95-slim-bookworm AS build
WORKDIR /src
COPY api/ ./
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    cargo build --release --locked -p mathesis-verifier
FROM build AS tested
RUN apt-get update && apt-get install -y --no-install-recommends libgmp10 \
    && rm -rf /var/lib/apt/lists/*
COPY --from=lean /opt/lean-workspace /opt/lean-workspace
COPY --from=lean /opt/verifier/.elan/toolchains /root/.elan/toolchains
# One proof of a Mathlib-only target that needs the prepared library of the
# workspace; the real Lean tests fail when the workspace declares a library and
# this proof is absent or no longer checks.
COPY infra/verifier/library-proof.json /opt/library-proof.json
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    WISHPOOL_TEST_LEAN_WORKSPACE=/opt/lean-workspace \
    WISHPOOL_TEST_LIBRARY_PROOF=/opt/library-proof.json \
    cargo test --release --locked -p mathesis-verifier real_lean -- --ignored --test-threads=1
FROM ${LEAN_IMAGE}
COPY --from=tested /src/target/release/mathesis-verifier /usr/local/bin/mathesis-verifier
USER 65532:65532
EXPOSE 8081
ENTRYPOINT ["/usr/local/bin/mathesis-verifier"]
