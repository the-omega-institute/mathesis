import type { RefereeRound, Submission } from '../api/types';
import { manuscriptReferences, resultLabel } from './statements';

export function formalizationAdvice(submission: Submission, round?: RefereeRound) {
  const candidates =
    round?.formalization_suggestions ??
    submission.formalization.items.map((item) => ({ claim: item.claim, plan: item.reason }));
  return candidates.flatMap((candidate) => {
    const claim = submission.claims.find(
      (item) =>
        item.id === candidate.claim &&
        item.has_proof &&
        item.kind !== 'conjecture' &&
        item.kind !== 'question',
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

export function formalizationAdviceText(submission: Submission, round?: RefereeRound): string {
  const candidates = formalizationAdvice(submission, round);
  const text = candidates.length
    ? candidates
        .map(
          ({ claim, plan, state }) =>
            `### ${resultLabel(claim)}\n\n${manuscriptReferences(plan, submission.claims)}${state ? `\n\n${{ proposed: 'Awaiting your approval.', approved: 'You have approved formalization.', in_progress: 'Formalization is in progress.', verified: 'A verified formal proof is available.', declined: 'You have declined formalization.' }[state]}` : ''}`,
        )
        .join('\n\n')
    : round?.formalization_suggestions
      ? 'No specific formalization recommendation is available from this review.'
      : 'No formalization recommendation is available from this review.';
  return `## Formalization opportunities\n\n${text}\n\n${submission.purpose === 'review_only' ? 'This private review does not schedule public formalization work.' : 'Recommendations do not schedule work. Public formalization requires author approval.'}`;
}
