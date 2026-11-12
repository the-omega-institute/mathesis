# §32. Accepted author-paper archive

Part of the repository contract in [CLAUDE.md](../../CLAUDE.md); equally binding.

The one ordinary public-submission checkbox records an exact
`public-archive-2026-11-12` marker and server time only when the final confirmation
sends it. The marker is bound to the confirmed source version and source digest.
Its absence preserves legacy and review-only behaviour. The submitting author may
grant the same one-way consent later in a cookie session for the current
publication-purpose version. A new version needs a new consent marker.

Consent means that an accepted version is displayed on Mathesis, licensed under CC
BY 4.0, copied by the publisher to its public repository and deposited in a
permanent public archive. The server stores no repository or archive credentials and
performs no external write.

An accepted public version with consent projects a licence, consent marker and time,
verified exact and concept DOIs when attached, and byte size, SHA-256 and download
URLs for its frozen PDF and source. The source download is public only in this state.
The public listing carries the current-version consent fact. Visibility changes do not
remove a stored citation.

An administrator attaches a DOI only after the shared `ArchiveRegistry::verify`
port checks the public archive record and both stored file hashes. The sidecar is
set once and bound to the accepted version. A public HTTPS archive-index file may
also request this same verification for accepted author records with consent and for
collected works; malformed entries are skipped and no unverified citation is stored.
