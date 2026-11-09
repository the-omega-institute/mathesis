//! Synchronize a reusable CMA environment to the independent verifier's pins.
use crate::{ReviewError, ReviewResult};

pub fn preparation(toolchain: &str) -> ReviewResult<String> {
    let environment =
        mathesis_verifier::Environment::parse(toolchain).map_err(ReviewError::Output)?;
    let version = environment
        .lean_toolchain
        .strip_prefix("leanprover/lean4:")
        .unwrap();
    let revision = &environment.mathlib_revision;
    let root = format!("/home/cma/mathesis-lean/{version}-{}", &revision[..12]);
    Ok(format!(
        "ENVIRONMENT REQUIRED BY THE INDEPENDENT VERIFIER: {toolchain}. Before proof work, acquire the shared flock /home/cma/mathesis-lean/compiler.lock. Inside the lock inspect actual available memory, disk, and processes whose comm basename is exactly lean or lake. Reuse {root} only if lean-toolchain and the resolved Mathlib Git revision match these exact pins and an import Mathlib compile succeeds. If absent or incompatible, automatically prepare this new version-specific directory using official public Lean/elan downloads and the public Mathlib repository at full revision {revision}; verify that Mathlib's lean-toolchain equals {}. Fetch the compiled cache for that exact revision. Use one compiler/cache extraction worker, measure resources before each phase, and avoid building all of Mathlib from source. Verify the installed lean --version and compile an import Mathlib readiness file before declaring the environment ready. Retain old version directories and other agents' work; never upgrade a directory in place. Reuse the verified environment for later papers and create a separate small proof directory for each task. A bootstrap failure is an environment problem, never evidence against the mathematical statement. Do not cancel, replay or replace an already admitted provider task just to change versions.\n",
        environment.lean_toolchain,
    ))
}
