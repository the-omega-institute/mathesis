import type { PublicClaim } from '../api/types';
import { CLAIM_KIND_LABELS } from './labels';

/** Preserve the manuscript's name/number, omitting its source-code label. */
export function resultLabel(
  claim: Pick<PublicClaim, 'label' | 'kind' | 'id'> & { section?: string },
): string {
  const label = claim.label.trim().replace(/\s*\[[^\]]*\]\s*$/, '');
  if (
    !label ||
    /^(theorem|lemma|proposition|corollary|conjecture|question|problem|claim|definition|remark)$/i.test(
      label,
    )
  ) {
    return label || CLAIM_KIND_LABELS[claim.kind];
  }
  return label;
}

type NamedResult = Parameters<typeof resultLabel>[0];

/** Translate known review references in prose; mathematical notation and source links are literal. */
export function manuscriptReferences(text: string, claims: readonly NamedResult[]): string {
  const names = new Map(claims.map((claim) => [claim.id, resultLabel(claim)]));
  const reference =
    /\b(?:(?:Statements?|Claims?|Theorems?|Lemmas?|Propositions?|Corollaries)\s+)?(C\d+)(?:\s*[-–—]\s*(C\d+))?\b/g;
  // Keep code, TeX, URLs and Markdown link destinations byte-for-byte intact.
  const literal =
    /(```[\s\S]*?```|`[^`\n]*`|\$\$[\s\S]*?\$\$|\$[^$\n]*\$|\\\([\s\S]*?\\\)|\\\[[\s\S]*?\\\]|https?:\/\/[^\s)\]]+|\]\([^)]*\))/g;
  return text
    .split(literal)
    .map((part, index) =>
      index % 2
        ? part
        : part.replace(reference, (original, first: string, last?: string) => {
            if (!names.has(first) || (last && !names.has(last))) return original;
            return last ? `${names.get(first)} through ${names.get(last)}` : names.get(first)!;
          }),
    )
    .join('');
}
