#!/usr/bin/env python3
"""Freeze Lake locks via anonymous GET, deriving all direct pins from infra/verifier.

Run after changing verifier inputs; this downloads JSON locks, never Lean sources.
"""
import hashlib
import json
from pathlib import Path
import re
import tomllib
import urllib.request
import urllib.error

ROOT = Path(__file__).resolve().parents[3]
VERIFIER = ROOT / "infra/verifier"
FILES = ("lean-toolchain", "lakefile.toml", "mathesis-libraries.json", "library-modules.txt", "Lean.Dockerfile")
inputs = {name: (VERIFIER / name).read_text() for name in FILES}
requirements = tomllib.loads(inputs["lakefile.toml"])["require"]
packages = {}
origins = []
for dep in requirements:
    assert re.fullmatch(r"[0-9a-f]{40}", dep["rev"])
    repo = dep["git"].removesuffix(".git").removeprefix("https://github.com/")
    assert re.fullmatch(r"[\w.-]+/[\w.-]+", repo)
    url = f"https://raw.githubusercontent.com/{repo}/{dep['rev']}/lake-manifest.json"
    request = urllib.request.Request(url, headers={"User-Agent": "mathesis-catalog-pins"})
    with urllib.request.urlopen(request, timeout=60) as response:
        raw = response.read()
    manifest = json.loads(raw)
    origins.append({"url": url, "sha256": hashlib.sha256(raw).hexdigest(), "manifest": manifest})
    for entry in manifest["packages"]:
        assert entry["type"] == "git" and re.fullmatch(r"[0-9a-f]{40}", entry["rev"])
        previous = packages.get(entry["name"])
        assert previous is None or (previous["rev"], previous["url"]) == (entry["rev"], entry["url"])
        packages[entry["name"]] = dict(entry, inherited=True)
for dep in requirements:
    repo = dep["git"].removesuffix(".git").removeprefix("https://github.com/")
    config_file = None
    for candidate in ("lakefile.toml", "lakefile.lean"):
        try:
            with urllib.request.urlopen(f"https://raw.githubusercontent.com/{repo}/{dep['rev']}/{candidate}", timeout=60) as response:
                response.read()
            config_file = candidate
            break
        except urllib.error.HTTPError as exc:
            if exc.code != 404:
                raise
    assert config_file is not None
    previous = packages.get(dep["name"])
    assert previous is None or previous["rev"] == dep["rev"]
    packages[dep["name"]] = {
        "name": dep["name"], "url": dep["git"], "rev": dep["rev"], "inputRev": dep["rev"],
        "type": "git", "subDir": None, "scope": "", "inherited": False,
        "manifestFile": "lake-manifest.json", "configFile": config_file,
    }
name = tomllib.loads(inputs["lakefile.toml"])["name"]
lock = {"version": "1.3.0", "packagesDir": ".lake/packages", "lakeDir": ".lake", "name": name,
        "fixedToolchain": False, "packages": [packages[n] for n in sorted(packages)]}
mathlib = next(dep for dep in requirements if dep["name"] == "mathlib")
mathlib_lock = dict(lock, packages=[p for p in lock["packages"] if p["name"] not in {d["name"] for d in requirements if d["name"] != "mathlib"}])
mathlib_lake = f'name = "{name}"\nversion = "0.1.0"\n\n[[require]]\nname = "mathlib"\ngit = "{mathlib["git"]}"\nrev = "{mathlib["rev"]}"\n'
value = {"schema": "mathesis.verifier-pins.v1", "inputs": inputs,
         "input_sha256": {n: hashlib.sha256(s.encode()).hexdigest() for n, s in inputs.items()},
         "lean_image_tag": hashlib.sha256("".join(inputs[n] for n in FILES).encode()).hexdigest()[:12],
         "toolchain": inputs["lean-toolchain"].strip() + ", Mathlib " + mathlib["rev"],
         "lake_manifest": lock, "mathlib_manifest": mathlib_lock, "mathlib_lakefile": mathlib_lake,
         "origins": origins}
(ROOT / "sdk/catalog/verifier-pins.json").write_text(json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2) + "\n")
