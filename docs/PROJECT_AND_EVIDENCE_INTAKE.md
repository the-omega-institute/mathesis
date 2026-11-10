# Lean projects and computational evidence

Mathesis accepts source projects independently of whether the current verifier can
check them. Depositing a project, passing static preflight and earning a Lean
receipt are separate events. None of the first two creates a verified claim.

## Full Lean projects

The web form accepts a project folder, ZIP or ordinary tar.gz, as well as a single
Lean file. Folder intake omits `.lake` and `.git`; source archives should omit build
caches. A single enclosing directory is removed when it contains `lean-toolchain`.
The source intake preserves up to 512 Lean modules / 8 MiB total as dotted names:
`Research.Helpers` means `Research/Helpers.lean`. Every source is an immutable
GridFS blob with its own digest. Case collisions, traversal and reserved roots
(Target, Solution, Check, Mathlib, Init, Lean, Std, Lake) are rejected.

`POST /api/v1/formalizations` accepts the reusable JSON project format. Optional
`project_files` preserves root `lean-toolchain`, `lake-manifest.json`,
`lakefile.lean`, `lakefile.toml`, `README.md`, `LICENSE`, `LICENSE.md` and
`LICENSE.txt` as inert text, with a 1 MiB combined limit. Other build products and
arbitrary resource files are not part of this Lean-source format. Include
computational data in a manuscript evidence bundle instead.

`solution` may be empty for a project deposit. A project with existing theorem
names can later supply a separate adapter without altering its original modules:

```lean
import Target
import Research.Helpers

theorem wishpool_solution : wishpool_target_prop := Research.existing_theorem
```

Send this as optional `solution` on
`POST /api/v1/formalizations/{id}/attempts` alongside `record`, `claim` and
`expected_target`. Each attempt binds the exact adapter and every submitted
module into its receipt. The original project remains downloadable at
`GET /api/v1/formalizations/{id}/project`; access is restricted to owner/staff.

Run `POST /api/v1/formalizations/preflight` with the same complete deposit body
before submission. It reports module count, source bytes, pinned dependencies,
missing imports, cycles, pin conflicts, unsupported constructs and whether a
Solution adapter is needed. Errors do not prevent source storage but prevent
verification admission. Static preflight cannot promise a successful compilation.

The checker compiles nested modules in dependency order in a fresh copy of an
operator-prepared workspace. Library modules may contain namespaces and import
specific Mathlib modules. Such imports must resolve to existing prepared artifacts.
Target and Solution remain strict root modules; all submitted modules, including
unused modules and comments, undergo the conservative unsafe-construct scan.
A verifier workspace may additionally declare prepared libraries whose modules
solutions and supporting modules import; preflight then accepts those imports and
reports a submitted module named below a library root
([LIBRARY_BINDING.md](LIBRARY_BINDING.md)).
The checker never runs uploaded Lake files or downloads dependencies. Custom
metaprogramming and unresolved dependencies can therefore be deposited but need
adaptation before verification. The exact target and standard axiom closure remain
mandatory. Historical single-file and project digest algorithms are unchanged.

## Computational evidence

Use “Computational evidence & supplements” on the initial source upload or revision
form. Give each file a type, an explanation of what it supports, and optional
statement labels, source URL and full Git revision. The browser packages these
with the manuscript. Ordinary `.tex`, ZIP and regular tar.gz sources are supported;
when adding attachments to a tar with extended headers, repack it as ZIP first.
Previously declared attachments in a source ZIP are retained, and duplicate paths
are rejected rather than overwritten.

API clients include a root `mathesis-evidence.json` in the source ZIP/tar archive:

```json
{
  "schema_version": 1,
  "files": [
    {
      "path": "evidence/certificate.json",
      "kind": "certificate",
      "description": "Finite case certificate supporting thm:main",
      "claims": ["thm:main"],
      "source_url": "https://github.com/owner/research"
    }
  ]
}
```

Kinds are `certificate`, `computation`, `source`, `data`, and `supplement`.
`source_revision`, when present, must be a full 40- or 64-hex-digit Git commit.
The server requires every declared file, derives SHA-256 and byte count from actual
bytes, and records them in `PaperVersion.evidence`. The manifest cannot supply its
own hashes. Maximum 100 attachments; existing 30 MiB compressed / 64 MiB expanded
source limits apply. The manifest itself is limited to 256 KiB. Paths are distinct
relative paths; ZIP symlinks and duplicate archive paths are refused.

Files stay inside that immutable source version. Uploading a new version does not
silently carry old attachments forward: include the evidence for the new version
in its source bundle. Older versions retain their original inventory and downloads.
The private download route is
`GET /api/v1/submissions/{id}/evidence?version=1&path=evidence%2Fcertificate.json`,
using the same author/staff source access rules. The review dossier exposes
`evidence_by_version`, alongside each review round's source hash.

The PDF referee receives the inventory and complete UTF-8 evidence files up to
64 KiB each / 256 KiB combined. Binary and oversized files are explicitly marked
`inventory_only_binary_or_size_limit`. For bundles declaring an evidence manifest, the Orchestra source adapter likewise sends
all file hashes and a bounded text payload (512 KiB each / 2 MiB combined), including
computational evidence. These limits describe transport coverage, not validation.
Manuscripts without declared evidence retain the exact legacy prompt representation,
so already admitted review tasks keep their input identity. Uploaded code is not automatically executed. Models must distinguish supplied
certificates from independently checked results, and unread files from read ones.

## Long manuscripts

Confirmation uses five statements per page, search by label/section/text, direct
statement navigation and explicit checked progress. All edits remain in page state
across navigation and all rows are submitted together. Manuscripts with more than
ten statements require each row to be checked before starting review. Editing a
row clears its check. Leaving or reloading the page does not persist unsaved edits.
Paper titles use safe inline KaTeX rendering, including titles in archive listings.
