# Legacy reviewed-note migration, 5 November 2026

The 36 previously delivered `trureturing-outreach` manuscripts entered the
ordinary submission channel before trusted collection intake existed. All 36
had a completed scientific result and an independent producer PASS bound to the
exact result and workflow. They do not need another Mathesis referee run.

The owner withdrew the 26 drafts and 6 active reviews, reading back each state.
The 4 terminal `not_accepted` records were retained. Original submissions,
reports and receipts remain private and are not reclassified as venue acceptance.

The operator consumed the existing `outreach-control` ledger snapshot read-only
and the exact scientific assets from each existing task release. For each note:

- Verify completed result, original PASS, matching task/workflow/result digest,
  and the original publication authorization.
- Check the reviewed scientific manifest, ordered authors, title, abstract,
  stable work ID, revision, CC BY 4.0 license and pinned Lean source.
- Verify the released PDF and source ZIP lengths and SHA-256 bindings against
  both the reviewed result and scientific manifest. Only scientific files enter
  collection; correspondence, prompts, raw task logs and private reviews do not.
- Deliver those unchanged PDF/source bytes to `/api/v1/pre-reviewed-deliveries`.
  Retain the actual original PASS and delivery evidence in the private review
  record under `mathesis.legacy-reviewed-selection.v1`. This historical migration
  makes no new prior-work verdict or independent Mathesis acceptance claim.
- Read back the public metadata and exact-version receipt, retaining the original
  scientific identity and hashes. Preserve existing full bilingual reading guides.
  The remaining guides use the existing independent exposition queue.

All 36 deliveries returned `collected`, with matching work/version/file digests;
all public metadata readbacks matched. Together with two newer producer deliveries,
the live collection now contains 38 distinct works. The historical ten are a
subset and must appear only once. Newer live versions take precedence over
historical snapshots; pinned historical pages remain available.

Private selections, scientific manifests, withdrawal confirmations, delivery
receipts and readbacks are archived outside the repository at
`mathesis-evidence/2026-11-05-legacy-intake`. No upstream ledger or producer code
was changed. Exact published deliveries remain immutable; later scientific
changes must use a new version and name the current collected predecessor.

During this reconciliation, existing dynamic PDF/source requests were found to
stall while metadata remained healthy (issue #83). File readback is a separate
required completion check; collection receipts alone do not prove download health.
