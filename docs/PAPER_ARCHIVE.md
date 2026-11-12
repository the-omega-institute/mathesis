# Accepted paper archive

The ordinary public submission choice has one meaning after acceptance. The
accepted version is displayed on Mathesis, licensed under CC BY 4.0, copied to
a public repository in the Mathesis papers organization, and deposited in a
permanent public archive with a DOI. The licence is [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/).
The repository and archive record remain available when the author later makes
the Mathesis page private. Reviews, letters and advice remain private.

The final submission may send the exact marker
`public-archive-2026-11-12`. Layer 2 stores the marker and server time on the
source version and its SHA-256. A missing marker means that the client did not
grant archive consent. The server never derives consent from `purpose`,
visibility, acceptance, or a legacy document. Stored documents without the new
fields load unchanged.

The submitting author may grant consent later with a cookie session. The
request names the same marker and applies only to the current version of a
publication-purpose submission. Consent cannot be withdrawn through the API.
A new manuscript version has no consent until the author grants it for that
version. Changing Mathesis page visibility does not delete a verified DOI.

## Public projection

An accepted public paper with consent has an `archive` object in
`GET /api/v1/papers/{record}`. It contains `licence: "CC-BY-4.0"`, the consent
time, the marker, an exact-version DOI when one has been verified, an optional
concept DOI, and descriptors for the frozen PDF and source. Each descriptor
contains the byte size, SHA-256 and download URL. The source download is
`GET /api/v1/papers/{record}/source`. It returns the exact stored source bytes.
The route returns `not_found` when the paper is not accepted and public or the
current version has no consent. The existing public PDF route keeps its prior
accepted-public behaviour.

`GET /api/v1/papers` lists accepted public records with complete pagination.
Every public listing row has `archive_consent`, which says whether the current
accepted version carries consent. No review, audit, letter, advice or consent
from another version is projected.

Collected-paper `pdf` and `source` descriptors now include the stored byte size
and SHA-256 in addition to their existing URL. No delivery digest, receipt or
stored document is changed.

## Verified DOI

`POST /api/v1/papers/{record}/archive` is an administrator route. The body is:

```json
{
  "doi": "10.5281/zenodo.123",
  "pdf_sha256": "<64 lowercase hexadecimal characters>",
  "source_sha256": "<64 lowercase hexadecimal characters>"
}
```

Layer 2 requires archive consent for the exact accepted version and checks that
the supplied digests match the stored PDF and source. It then calls the shared
`ArchiveRegistry::verify` port. The registry reads the public archive record,
requires the published exact-version DOI, and checks both archived file hashes
against the stored bytes. A concept DOI is retained separately. A successful
attachment is set once and bound to the version. Repeating the same DOI is
idempotent. A different DOI, a missing consent marker, a missing PDF, a digest
mismatch, an unpublished record, or different archived bytes is rejected.
Mathesis never registers a DOI.

## Archive index discovery

The optional `WISHPOOL_ARCHIVE_INDEX_URL` setting enables worker discovery. It
must be an HTTPS URL without credentials, query or fragment. An unset value
disables discovery. The worker sends no credentials, follows no redirects, has
a 30-second timeout and accepts at most 1 MiB.

The publisher commits this exact JSON shape to the organization catalog:

```json
{ "schema": "mathesis.archive-index.v1",
  "generated_at": "<RFC 3339>",
  "entries": [ { "kind": "record" | "collected",
                 "id": "<record id or work id>",
                 "version": <positive integer>,
                 "doi": "10.5281/zenodo.<n>" } ] }
```

The schema string and RFC 3339 timestamp are required. Unknown fields are
ignored. Malformed entries are ignored individually. For each remaining entry,
Layer 2 checks that the record or collected work and exact version exist, that
no DOI is already attached, and that an author record has archive consent for
that version. It then runs the shared archive verification and attaches the
citation only after both file hashes match. Unknown records, wrong versions,
missing consent and already attached citations are skipped. An unavailable or
unverifiable archive is logged and retried on a later reconciliation cycle.
Nothing is overwritten.

The server only reads this public index and the public archive record. It does
not create GitHub repositories, write to GitHub or Zenodo, register DOIs, or
hold GitHub or Zenodo credentials. The organization publisher owns repository
creation and archive deposits.
