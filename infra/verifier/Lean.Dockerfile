FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl git libgmp10 zstd \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --uid 65532 --create-home --home-dir /opt/verifier verifier \
    && mkdir -p /opt/lean-workspace && chown verifier:verifier /opt/lean-workspace
USER 65532:65532
ENV HOME=/opt/verifier
ENV PATH=/opt/verifier/.elan/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
WORKDIR /opt/lean-workspace
COPY --chown=65532:65532 infra/verifier/lean-toolchain infra/verifier/lakefile.toml \
    infra/verifier/mathesis-libraries.json infra/verifier/library-modules.txt ./
# Mathlib comes from its build cache. The prepared library (docs/LIBRARY_BINDING.md)
# is compiled here from source:
# - It is fetched at the exact revision of lakefile.toml with only its Lean
#   sources checked out. Lake finds the package already at that revision, records
#   it in lake-manifest.json and fetches nothing further.
# - Only the modules of library-modules.txt and their imports are compiled. A
#   module that does not compile at the pinned revision fails the image build.
# - The compiler's output goes to a file: the library's linter messages exceed
#   the 2 MiB step log of the image builder. The step prints one progress line a
#   minute, and on failure the failing modules with their errors.
# - Verification reads compiled modules only, and every job copies the workspace,
#   so sources, Git data and intermediate C files are removed in the same layer.
RUN set -eu; \
    library=.lake/packages/trureturing; \
    library_field() { \
        awk -v field="$1" '/^\[\[require\]\]/ { found = 0 } \
            $0 == "name = \"trureturing\"" { found = 1 } \
            found && $1 == field { gsub(/"/, "", $3); print $3 }' lakefile.toml; \
    }; \
    url=$(library_field git); revision=$(library_field rev); \
    echo "$revision" | grep -Eq '^[0-9a-f]{40}$'; \
    modules=$(sed -e 's/#.*//' -e 's/[[:space:]]//g' -e '/^$/d' library-modules.txt); \
    test -n "$modules"; \
    curl -fsSL https://github.com/leanprover/elan/releases/download/v4.2.1/elan-x86_64-unknown-linux-gnu.tar.gz -o /tmp/elan.tar.gz; \
    tar -xzf /tmp/elan.tar.gz -C /tmp; \
    /tmp/elan-init -y --no-modify-path --default-toolchain none; \
    elan toolchain install "$(cat lean-toolchain)"; \
    rm /tmp/elan.tar.gz /tmp/elan-init; \
    git init -q "$library"; \
    git -C "$library" remote add origin "$url"; \
    git -C "$library" fetch -q --depth 1 --filter=blob:none origin "$revision"; \
    git -C "$library" sparse-checkout set --cone D5; \
    git -C "$library" checkout -q --detach "$revision"; \
    lake update; \
    lake exe cache get; \
    test -f .lake/packages/mathlib/.lake/build/lib/lean/Mathlib.olean; \
    test "$(git -C "$library" rev-parse HEAD)" = "$revision"; \
    started=$(date +%s); \
    lake build $(printf '+%s ' $modules) > /tmp/library-build.log 2>&1 & \
    build=$!; \
    while kill -0 "$build" 2>/dev/null; do \
        sleep 60; \
        echo "prepared library: $(grep -c '\] Built ' /tmp/library-build.log) modules built after $(( $(date +%s) - started )) s"; \
    done; \
    if ! wait "$build"; then \
        grep -E -A 15 '^(✖|error:)' /tmp/library-build.log | head -n 500; \
        tail -n 30 /tmp/library-build.log; \
        exit 1; \
    fi; \
    echo "prepared library: compiled in $(( $(date +%s) - started )) s, $(grep -c '^warning:' /tmp/library-build.log) warnings"; \
    rm /tmp/library-build.log; \
    for module in $modules; do \
        test -f "$library/.lake/build/lib/lean/$(echo "$module" | tr . /).olean"; \
    done; \
    find "$library" -mindepth 1 -maxdepth 1 ! -name .lake -exec rm -rf {} +; \
    find "$library/.lake" -mindepth 1 -maxdepth 1 ! -name build -exec rm -rf {} +; \
    find "$library/.lake/build" -mindepth 1 -maxdepth 1 ! -name lib -exec rm -rf {} +; \
    echo "prepared library: $(find "$library/.lake/build/lib/lean" -name '*.olean' | wc -l) modules, $(du -sm "$library" | cut -f1) MiB"
