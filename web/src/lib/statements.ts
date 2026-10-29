import type { PublicClaim } from '../api/types';
import { CLAIM_KIND_LABELS } from './labels';

/** Preserve the manuscript's name/number, omitting its source-code label. */
export function resultLabel(claim: Pick<PublicClaim, 'label' | 'kind' | 'id'>): string {
  return (
    claim.label.trim().replace(/\s*\[[^\]]*\]\s*$/, '') ||
    `${CLAIM_KIND_LABELS[claim.kind]} ${claim.id}`
  );
}
