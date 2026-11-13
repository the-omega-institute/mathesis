#!/usr/bin/env python3
"""Compile separate targets or one exact bundle. No verification receipt is issued.

Dependencies must already be prepared. Never executes contributor Lake scripts.
"""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def read(path):
    return json.loads(path.read_text())


def require(condition, message):
    if not condition:
        raise ValueError(message)


def without_comments(text):
    return re.sub(r"--[^\n]*", "", text)  # Block comments handled below, including nesting.


def imports(text):
    depth, i, out = 0, 0, []
    while i < len(text):
        if text[i:i+2] == "/-":
            depth += 1
            i += 2
        elif depth and text[i:i+2] == "-/":
            depth -= 1
            i += 2
        else:
            out.append("\n" if text[i] == "\n" else " " if depth else text[i])
            i += 1
    clean = without_comments("".join(out))
    return [name for line in clean.splitlines() if line.strip().startswith("import ") for name in line.strip()[7:].split()]


def build(unit, workspace):
    require((workspace / "lean-toolchain").read_text().strip() == unit["toolchain"].split(", Mathlib ")[0], "workspace Lean pin differs")
    packages = {p["name"]: p for p in read(workspace / "lake-manifest.json")["packages"]}
    require(packages["mathlib"]["rev"] == unit["toolchain"].split(", Mathlib ")[1], "workspace Mathlib pin differs")
    for lib in unit.get("libraries") or []:
        require(lib["name"] in packages and packages[lib["name"]]["rev"] == lib["revision"], "workspace prepared-library pin differs")
    path = ROOT / unit["path"]
    require(path.resolve().is_relative_to(ROOT), "build path escapes repository")
    if unit["kind"] == "bundle":
        request = read(path / "bundle.json")["verification_request"]
        sources = {"Target": request["target"], "Solution": request["solution"], **request.get("modules", {})}
    else:
        sources = {"Target": path.read_bytes().decode("utf-8")}
    pending, done = dict(sources), set()
    with tempfile.TemporaryDirectory(prefix="mathesis-modules-") as directory:
        scratch = Path(directory)
        for name, source in sources.items():
            require(re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*(\.[A-Za-z_][A-Za-z0-9_]*)*", name), "invalid module name")
            dest = scratch / (name.replace(".", "/") + ".lean")
            dest.parent.mkdir(parents=True, exist_ok=True)
            dest.write_bytes(source.encode())
        env = dict(os.environ, LEAN_PATH=str(scratch) + (os.pathsep + os.environ["LEAN_PATH"] if os.environ.get("LEAN_PATH") else ""))
        while pending:
            ready = sorted(name for name, text in pending.items() if all(dep not in sources or dep in done for dep in imports(text)))
            require(ready, "submitted module import cycle")
            for name in ready:
                file = scratch / (name.replace(".", "/") + ".lean")
                # -o puts each artifact inside this build unit's temporary root.
                subprocess.run(["lake", "env", "lean", "--root=" + str(scratch), "-o", str(file.with_suffix(".olean")), str(file)], cwd=workspace, env=env, check=True)
                done.add(name)
                del pending[name]
    print("Compiled " + unit["path"] + " (no Mathesis verification or credit)", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("unit", help="targets, or an exact path from index/modules.json")
    parser.add_argument("--workspace", type=Path, help="existing prepared workspace; no dependency installation")
    args = parser.parse_args()
    units = read(ROOT / "index/modules.json")["modules"]
    if args.unit == "targets":
        selected = [u for u in units if u["kind"] != "bundle" and u["compatible"]]
        skipped = [u["path"] for u in units if u["kind"] != "bundle" and not u["compatible"]]
        for path in skipped:
            print("Skipped historical pin: " + path)
    else:
        selected = [u for u in units if u["path"] == args.unit]
        require(len(selected) == 1, "choose an exact path in index/modules.json")
    for unit in selected:
        workspace = (args.workspace or (ROOT if unit["kind"] == "bundle" else ROOT / "ci/mathlib")).resolve()
        build(unit, workspace)
    print(f"Compiled {len(selected)} independent build units. Proof bundles are compiled only when individually selected.")


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, KeyError, IndexError, subprocess.CalledProcessError) as exc:
        sys.exit(f"Build failed: {exc}")
