#!/usr/bin/env python3
"""Publish explicitly selected, upstream-reviewed scientific note packages.

Consumes existing coordinator selections and short-note.v1 artifacts. It never
reads the Mathesis review database or copies coordinator evidence into the site.
Publication is an operator action; ordinary manuscript decisions are unchanged.
"""
import argparse
from datetime import date
import hashlib
import json
import re
import shutil
from pathlib import Path
from urllib.parse import urlparse


def digest(value):
    return hashlib.sha256(value).hexdigest()


def canonical(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True,
                      separators=(",", ":"), allow_nan=False).encode()


def checked_file(folder, artifact):
    name = artifact["name"]
    if not name or name in (".", "..") or Path(name).name != name or "\\" in name:
        raise ValueError("artifact must be a single filename")
    path = folder / name
    if path.is_symlink():
        raise ValueError("scientific artifacts must be regular files")
    content = path.read_bytes()
    if len(content) != artifact["bytes"] or digest(content) != artifact["sha256"]:
        raise ValueError("artifact does not match the reviewed package: " + name)
    return path


def public_url(value):
    url = urlparse(value)
    if url.scheme != "https" or not url.hostname or url.username or url.password:
        raise ValueError("source link must be a public HTTPS URL")
    return value


def prepare(selection_file, artifact_root, published_on):
    selection = json.loads(selection_file.read_text())
    result, review, envelope = (selection[k] for k in ("result", "review", "envelope"))
    if result["outcome"] != "completed" or review["verdict"] != "PASS":
        raise ValueError("only completed, upstream-approved packages can be indexed")
    result_hash = digest(canonical(result))
    if result_hash != envelope["result_hash"] or result_hash != review["binding"]["result_hash"]:
        raise ValueError("upstream review is not bound to this package")
    for key in ("task_id", "workflow_hash"):
        if review["binding"][key] != envelope[key]:
            raise ValueError("upstream review identity mismatch")
    paper = envelope["metadata"]["paper"]
    if paper["purpose"] != "publication" or paper["make_public_after_acceptance"] is not True:
        raise ValueError("private-review packages cannot enter a public collection")
    work_id = envelope["metadata"]["work_id"]
    if not re.fullmatch(r"[a-z0-9][a-z0-9-]{0,159}", work_id):
        raise ValueError("invalid work identifier")
    artifacts = result["artifacts"]
    manifests = [a for a in artifacts if re.fullmatch(r"note-v\d+\.json", a["name"])]
    folder = artifact_root / work_id
    if len(manifests) == 1:
        note = json.loads(checked_file(folder, manifests[0]).read_text())
        if note["schema_version"] != "short-note.v1":
            raise ValueError("unsupported note package")
        meta = note["metadata"]
    elif not manifests:
        handoffs = [a for a in artifacts if a["name"] == "handoff.json"]
        if len(handoffs) != 1:
            raise ValueError("one reviewed scientific manifest is required")
        note = json.loads(checked_file(folder, handoffs[0]).read_text())
        if note["schema_version"] != "outreach-mathesis-handoff.v1":
            raise ValueError("unsupported handoff package")
        meta = note["source_note"]
    else:
        raise ValueError("ambiguous note manifests")
    if type(meta["revision"]) is not int or meta["revision"] < 1:
        raise ValueError("revision must be a positive integer")
    if result["task_id"] != envelope["task_id"]:
        raise ValueError("result task identity mismatch")
    if meta["id"] != work_id or str(meta["revision"]) != envelope["metadata"]["version"]:
        raise ValueError("note identity/version differs from the selected delivery")
    if meta["creators"] != paper["authors"] or meta["title"] != paper["title"]:
        raise ValueError("reviewed authors or title differ from the selected delivery")
    if meta["license"].lower() != "cc-by-4.0":
        raise ValueError("this collection importer requires CC BY 4.0")
    authors = [{k: a[k] for k in ("name", "affiliation", "orcid") if k in a}
               for a in meta["creators"]]
    revision = str(meta["revision"])
    files, copies = {}, []
    for role, field in (("note-pdf", "pdf"), ("source-archive", "source")):
        matches = [a for a in artifacts if a["role"] == role
                   and (field != "source" or a["name"] == envelope["source"]["name"])]
        if len(matches) != 1:
            raise ValueError("exactly one " + role + " is required")
        artifact = matches[0]
        if field == "source" and any(artifact[k] != envelope["source"][k]
                                     for k in ("name", "bytes", "sha256")):
            raise ValueError("source artifact differs from the selected delivery")
        source = checked_file(folder, artifact)
        if note["files"].get(artifact["name"]) != artifact["sha256"]:
            raise ValueError("short-note manifest does not bind scientific file")
        destination = f"/collections/trureturing/{work_id}/v{revision}/{artifact['name']}"
        files[field] = {"url": destination, "sha256": artifact["sha256"]}
        copies.append((source, destination))
    proof = public_url(meta["evidence_url"])
    match = re.fullmatch(r"https://github\.com/the-omega-institute/trureturing/blob/([a-f0-9]{40})/(.+\.lean)(?:#L\d+)?", proof)
    if not match:
        raise ValueError("Lean evidence needs an exact public source revision")
    confirmation = json.loads((selection_file.parent / "checked-confirmation.json").read_text())
    if confirmation["source_sha256"] != envelope["source"]["sha256"]:
        raise ValueError("original problem metadata is not bound to the selected source")
    problems = []
    for claim in confirmation["claims"]:
        problem = claim.get("settles")
        if problem and not claim.get("excluded") and problem not in problems:
            source = problem["source"]
            if source["kind"] not in ("arxiv", "doi", "oeis", "zenodo", "url", "named_work", "personal", "hexagon"):
                raise ValueError("unsupported original source")
            locator = source["locator"]
            if source["kind"] == "url":
                public_url(locator)
            problems.append({"name": problem["name"], "source": {
                k: source[k] for k in ("kind", "locator", "year") if k in source}})
    # Display only scientific metadata, never raw review reasons, private URLs,
    # account identities, delivery receipts, signatures, or execution history.
    work = {"id": work_id, "revision": revision, "title": meta["title"],
            "abstract": meta["abstract"], "summary": meta["summary"],
            "authors": authors, "license": "CC-BY-4.0", "published_on": published_on,
            "problem_ids": meta["problem_ids"], "problems": problems,
            "lean": {"url": proof, "revision": match[1]},
            "review": {"basis": "upstream_manuscript_review", "source": "Trureturing research workflow"},
            **files}
    return work, copies


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--selections", required=True, type=Path)
    parser.add_argument("--artifacts", required=True, type=Path)
    parser.add_argument("--published-on", required=True)
    parser.add_argument("--authorize-publication", action="store_true", required=True)
    parser.add_argument("--web", type=Path, default=Path(__file__).resolve().parents[2] / "web")
    args = parser.parse_args()
    date.fromisoformat(args.published_on)
    if not re.fullmatch(r"\d{4}-\d{2}-\d{2}", args.published_on):
        raise ValueError("publication date must be YYYY-MM-DD")
    path = args.web / "src/data/project-notes.json"
    existing = json.loads(path.read_text()) if path.exists() else {"schema": "mathesis.project-notes.v1", "works": []}
    works = {(w["id"], w["revision"]): w for w in existing["works"]}
    copies = []
    # Validate the complete batch before writing any public file.
    selections = sorted(args.selections.glob("*/selection.json"))
    if not selections:
        raise ValueError("no selected notes")
    for selection in selections:
        work, files = prepare(selection, args.artifacts, args.published_on)
        key = (work["id"], work["revision"])
        if key in works:
            work["published_on"] = works[key]["published_on"]
            # Exposition is maintained by the downstream reader. An exact
            # scientific-package retry must preserve its independently written guide.
            if "plain_language" in works[key]:
                work["plain_language"] = works[key]["plain_language"]
            if work != works[key]:
                raise ValueError("a published revision is immutable: " + work["id"])
        works[key] = work
        copies.extend(files)
    for source, url in copies:
        destination = args.web / "public" / url.lstrip("/")
        if destination.exists() and source.read_bytes() != destination.read_bytes():
            raise ValueError("refusing to overwrite a published artifact")
    for source, url in copies:
        destination = args.web / "public" / url.lstrip("/")
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, destination)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps({"schema": existing["schema"], "works": sorted(works.values(), key=lambda w: (w["id"], int(w["revision"])))}, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps({"notes": len({w["id"] for w in works.values()}), "versions": len(works), "scientific_files": len(copies)}))


if __name__ == "__main__":
    main()
