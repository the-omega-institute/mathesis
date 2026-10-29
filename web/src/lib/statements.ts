import type { PublicClaim } from '../api/types';
import { CLAIM_KIND_LABELS } from './labels';

/** Preserve the manuscript's name/number, omitting its source-code label. */
export function resultLabel(claim: Pick<PublicClaim, 'label' | 'kind' | 'id' | 'section'>): string {
  const label = claim.label.trim().replace(/\s*\[[^\]]*\]\s*$/, '');
  if (
    !label ||
    /^(theorem|lemma|proposition|corollary|conjecture|question|problem|claim|definition|remark)$/i.test(
      label,
    )
  ) {
    const kind = label || CLAIM_KIND_LABELS[claim.kind];
    return claim.section?.trim()
      ? `${kind}: ${claim.section.trim()} (${claim.id})`
      : `${kind} ${claim.id}`;
  }
  return label;
}
