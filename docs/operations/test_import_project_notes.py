"""Importer boundary tests; run with python3 -m unittest discover -s docs/operations."""
import copy
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from import_project_notes import canonical, digest, prepare


class ProjectImportTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.selection_dir = self.root / "selections" / "test-note"
        self.artifacts = self.root / "artifacts"
        folder = self.artifacts / "test-note"
        folder.mkdir(parents=True)
        self.selection_dir.mkdir(parents=True)
        scientific = {"note-v1.pdf": b"%PDF-public-science", "source-v1.zip": b"public-source"}
        meta = {"id": "test-note", "revision": 1, "title": "A result", "creators": [
            {"name": "An Author", "affiliation": "Institute", "private_account": "SECRET"}],
            "license": "cc-by-4.0", "abstract": "An abstract", "summary": "A summary",
            "problem_ids": ["original-question"], "evidence_url":
            "https://github.com/the-omega-institute/trureturing/blob/" + "a" * 40 + "/Proof.lean#L10"}
        manifest = {"schema_version": "short-note.v1", "metadata": meta,
                    "files": {k: digest(v) for k, v in scientific.items()}}
        scientific["note-v1.json"] = canonical(manifest)
        artifacts = []
        for name, content in scientific.items():
            (folder / name).write_bytes(content)
            artifacts.append({"name": name, "bytes": len(content), "sha256": digest(content),
                              "role": "note-pdf" if name.endswith(".pdf") else "source-archive" if name.endswith(".zip") else "manifest"})
        result = {"outcome": "completed", "task_id": "PRIVATE-TASK", "artifacts": artifacts,
                  "report": "SECRET PRIVATE REVIEW"}
        self.selection = {"result": result, "review": {"verdict": "PASS", "binding": {
            "result_hash": digest(canonical(result)), "task_id": "PRIVATE-TASK", "workflow_hash": "workflow"}},
            "envelope": {"result_hash": digest(canonical(result)), "task_id": "PRIVATE-TASK",
                "workflow_hash": "workflow", "source": artifacts[1], "metadata": {
                    "work_id": "test-note", "version": "1", "paper": {"purpose": "publication",
                        "make_public_after_acceptance": True, "authors": meta["creators"], "title": meta["title"]}}}}
        self.write_selection()
        confirmation = {"source_sha256": artifacts[1]["sha256"], "claims": [{"settles": {
            "name": "Original Poser's question", "source": {"kind": "oeis", "locator": "A123456", "private_note": "SECRET"},
            "internal": "SECRET"}}]}
        (self.selection_dir / "checked-confirmation.json").write_text(json.dumps(confirmation))

    def write_selection(self):
        (self.selection_dir / "selection.json").write_text(json.dumps(self.selection))

    def run_import(self, *extra):
        return subprocess.run([sys.executable, str(Path(__file__).with_name("import_project_notes.py")),
            "--selections", str(self.selection_dir.parent), "--artifacts", str(self.artifacts),
            "--web", str(self.root / "web"), "--published-on", "2026-11-04", *extra], capture_output=True)

    def test_publishes_only_science_and_preserves_original_credit(self):
        result = self.run_import("--authorize-publication")
        self.assertEqual(result.returncode, 0, result.stderr)
        snapshot = self.root / "web/src/data/project-notes.json"
        public = snapshot.read_text()
        self.assertNotIn("SECRET", public)
        self.assertNotIn("PRIVATE-TASK", public)
        work = json.loads(public)["works"][0]
        self.assertEqual(work["problems"][0]["name"], "Original Poser's question")
        self.assertEqual(work["authors"][0]["name"], "An Author")
        self.assertEqual(work["review"]["basis"], "upstream_manuscript_review")
        self.assertEqual(len(list((self.root / "web/public").rglob("*.*"))), 2)
        self.assertEqual(self.run_import("--authorize-publication").returncode, 0)

    def test_requires_deliberate_publication_action(self):
        self.assertNotEqual(self.run_import().returncode, 0)
        self.assertFalse((self.root / "web").exists())

    def test_rejects_unapproved_private_or_unbound_packages(self):
        original = copy.deepcopy(self.selection)
        for mutate in [
            lambda s: s["review"].update(verdict="FAIL"),
            lambda s: s["result"].update(report="different review"),
            lambda s: s["review"]["binding"].update(task_id="wrong-task"),
            lambda s: s["envelope"]["metadata"]["paper"].update(make_public_after_acceptance=False),
            lambda s: s["envelope"]["metadata"].update(version="2"),
        ]:
            self.selection = copy.deepcopy(original)
            mutate(self.selection)
            self.write_selection()
            with self.assertRaises(ValueError):
                prepare(self.selection_dir / "selection.json", self.artifacts, "2026-11-04")

    def test_rejects_tampering_before_any_public_write(self):
        (self.artifacts / "test-note/note-v1.pdf").write_bytes(b"tampered")
        self.assertNotEqual(self.run_import("--authorize-publication").returncode, 0)
        self.assertFalse((self.root / "web").exists())

    def test_preserves_published_versions_and_bytes(self):
        self.assertEqual(self.run_import("--authorize-publication").returncode, 0)
        snapshot = self.root / "web/src/data/project-notes.json"
        existing = json.loads(snapshot.read_text())
        existing["works"][0]["title"] = "Previously published title"
        snapshot.write_text(json.dumps(existing))
        before = snapshot.read_bytes()
        self.assertNotEqual(self.run_import("--authorize-publication").returncode, 0)
        self.assertEqual(snapshot.read_bytes(), before)


if __name__ == "__main__":
    unittest.main()
