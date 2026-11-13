#!/usr/bin/env python3
"""Check the public projection offline. This does not run Lean or confer credit."""
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]


def read(path):
    return json.loads((ROOT / path).read_text(encoding="utf-8"))


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(text):
    return hashlib.sha256(text.encode()).hexdigest()


def source(path):
    file = ROOT / path
    require(not file.is_symlink() and file.resolve().is_relative_to(ROOT), f"unsafe path: {path}")
    return file.read_bytes().decode("utf-8")


def check():
    manifest = read(".mathesis/generated.json")
    require(manifest["schema"] == "mathesis.formalization-generated.v1", "unknown manifest")
    for path, expected in manifest["sha256"].items():
        require(digest(source(path)) == expected, f"generated hash mismatch: {path}")
    pins = read("environment/verifier-pins.json")
    require(read("lake-manifest.json") == pins["lake_manifest"], "root Lake manifest differs")
    require(read("ci/mathlib/lake-manifest.json") == pins["mathlib_manifest"], "Mathlib lock differs")
    require(source("lakefile.toml") == pins["inputs"]["lakefile.toml"], "Lake file differs")
    require(source("lean-toolchain") == pins["inputs"]["lean-toolchain"], "toolchain differs")
    units = read("index/modules.json")["modules"]
    paths = [u["path"] for u in units]
    require(len(paths) == len(set(p.lower() for p in paths)), "duplicate build path")
    counts = {k: 0 for k in read("snapshot.json")["counts"]}
    for unit in units:
        require(unit["compatible"] == (unit["toolchain"] == pins["toolchain"]), "incorrect compatibility")
        path = unit["path"]
        if unit["kind"] != "bundle":
            require(digest(source(path)) == unit["digest"], f"target digest mismatch: {path}")
            continue
        bundle = read(path + "/bundle.json")
        request, attempt = bundle["verification_request"], bundle["attempt"]
        receipt = attempt["receipt"]
        require(read(path + "/receipt.json") == receipt, "receipt differs from bundle")
        require(source(path + "/Target.lean") == request["target"], "target differs from bundle")
        require(source(path + "/Solution.lean") == request["solution"] == attempt["solution"], "solution differs")
        require(digest(request["target"]) == receipt["target_digest"] == request["target_digest"], "bundle target digest differs")
        modules = request.get("modules", {})
        require(modules == attempt.get("modules", {}), "bundle modules differ")
        for name, text in modules.items():
            require(source(path + "/" + name.replace(".", "/") + ".lean") == text, "supporting source differs")
        hashed = (json.dumps(["mathesis-project-v1", request["solution"], dict(sorted(modules.items()))], ensure_ascii=False, separators=(",", ":"))
                  if modules else request["solution"])
        require(digest(hashed) == receipt["solution_digest"], "solution digest differs")
        require(unit["toolchain"] == request["toolchain"] == receipt["toolchain"], "bundle toolchain differs")
        require(receipt["verdict"] in ("proved", "disproved"), "unverified bundle")
        area = "proofs" if receipt["verdict"] == "proved" else "disproofs"
        require(path.startswith(area + "/"), "disproof/proof area mismatch")
        counts["proof_bundles" if area == "proofs" else "disproof_bundles"] += 1
    nodes = read("index/nodes.json")["nodes"]
    require(len({n["identity"] for n in nodes}) == len(nodes), "duplicate nodes")
    for node in nodes:
        counts[node["classification"]] += 1
        require(node["scope"] == "in_scope", "out-of-scope source publication")
        require(node["trureturing_write_back"] == {"request_url": None, "state": None, "revision": None}, "unsupported write-back inference")
        if node["classification"] == "no_target_yet":
            require(node["target_path"] is None and node["reason"] == node["next_step"], "missing API reason")
        else:
            require(node["target_path"] in paths, "node target omitted")
        for result in node["results"]:
            bundle = read(result["path"] + "/bundle.json")
            require((bundle["record"], bundle["claim"], bundle["version"]) == (node["record"], node["claim"], node["version"]), "bundle node identity differs")
            require(bundle["attempt"]["id"] == result["attempt"], "attempt identity differs")
    curated = read("index/curated.json")["targets"]
    for target in curated:
        require(target["open_for_submission"] is False and target["path"] in paths, "curated target scope differs")
    counts["curated_targets"] = len(curated)
    meta = read("snapshot.json")
    require(meta["nodes"] == len(nodes) and counts == meta["counts"], "counts differ from indexes")
    papers = list((ROOT / "index/papers").glob("*.json"))
    require(meta["papers"] == len(papers), "paper count differs")
    for path in papers:
        paper = json.loads(path.read_text())
        expected = [n for n in nodes if n["work"] == paper["work"]]
        actual = [n for group in paper["split"].values() for n in group]
        require(sorted(n["identity"] for n in expected) == sorted(n["identity"] for n in actual), "paper split differs")
    boards = read("leaderboard.json")["boards"]
    require(meta["leaderboard_rows"] == {k: len(v) for k, v in boards.items()}, "leaderboard counts differ")
    print(f"Projection hashes, exact sources, indexes and counts checked: {len(nodes)} nodes, {len(curated)} curated targets. Lean not run.")


if __name__ == "__main__":
    try:
        check()
    except (OSError, ValueError, KeyError) as exc:
        sys.exit(f"Projection check failed: {exc}")
