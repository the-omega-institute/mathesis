import type { Claim, RefereeRound, Submission } from '../api/types';
import { manuscriptReferences, resultLabel } from './statements';

export function formalizationAdvice(submission: Submission, round?: RefereeRound) {
  const candidates =
    round?.formalization_suggestions ??
    submission.formalization.items.map((item) => ({ claim: item.claim, plan: item.reason }));
  return candidates.flatMap((candidate) => {
    const claim = submission.claims.find(
      (item) =>
        item.id === candidate.claim && item.kind !== 'conjecture' && item.kind !== 'question',
    );
    if (!claim) return [];
    return [
      {
        claim,
        plan:
          candidate.plan +
          ('excerpt' in candidate && candidate.excerpt
            ? '\n\nThis is an excerpt of the recommendation.'
            : ''),
        state: submission.formalization.items.find((item) => item.claim === claim.id)?.state.state,
      },
    ];
  });
}

/** Source extraction is a hint; an audit can recognize a proof written in prose. */
export function hasReviewedProof(claim: Claim, round?: RefereeRound): boolean {
  if (claim.kind === 'conjecture' || claim.kind === 'question') return false;
  const reading =
    round?.audit?.state.state === 'done'
      ? round.audit.state.result.claims.find((item) => item.claim === claim.id)
      : undefined;
  return reading ? reading.correctness === 'correct' : claim.has_proof;
}

export function formalizationAdviceText(submission: Submission, round?: RefereeRound): string {
  const candidates = formalizationAdvice(submission, round);
  const text = candidates.length
    ? candidates
        .map(
          ({ claim, plan, state }) =>
            `### ${resultLabel(claim)}\n\n${manuscriptReferences(plan, submission.claims)}${state ? `\n\n${{ proposed: 'You can request this result be formalized in Lean.', approved: 'The exact Lean statement is being prepared.', in_progress: 'Proof work is in progress and will continue from saved results.', verified: 'An independently verified formal proof is available.', declined: 'You declined this formalization.' }[state]}` : ''}`,
        )
        .join('\n\n')
    : round?.formalization_suggestions
      ? 'No specific formalization recommendation is available from this review.'
      : 'No formalization recommendation is available from this review.';
  return `## Formalization opportunities\n\n${text}\n\n${submission.purpose === 'review_only' ? 'This private review does not schedule public formalization work.' : 'After you approve and confirm a public statement, Mathesis can work on its formal proof. A proof is shown as verified only after independent checking.'}`;
}

/** Existing manuscripts need no migration or repeated review to show saved target state. */
export function legacyFormalizationStatus(submission: Submission, claim: string): string {
  const item = submission.formalization.items.find((entry) => entry.claim === claim);
  if (!item || item.state.state === 'proposed' || item.state.state === 'declined') return '';
  if (item.state.state === 'verified') return 'A formal proof is recorded for this result.';
  if (submission.analysis_visibility !== 'public')
    return 'Proof work is paused while this paper is private.';
  const target = [...submission.lean_statements]
    .reverse()
    .find(
      (entry) =>
        entry.claim === claim &&
        entry.version === submission.versions.at(-1)?.number &&
        entry.claims_revision === submission.claims_revision,
    );
  if (target?.response.state === 'confirmed')
    return 'The Lean statement is confirmed. Proof work can continue from this target.';
  if (target?.response.state === 'awaiting_author')
    return 'The Lean statement is ready for your confirmation.';
  return 'The exact Lean statement is being prepared.';
}
