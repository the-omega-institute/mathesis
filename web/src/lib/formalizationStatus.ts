import type { Submission } from '../api/types';

type FormalizationStatus = { label: string; tone: 'good' | 'accent' | 'muted' };

/** A prepared Lean target and an independently verified proof are separate milestones. */
export function formalizationStatus(submission: Submission, claim: string): FormalizationStatus {
  const item = submission.formalization.items.find((item) => item.claim === claim);
  if (item?.state.state === 'verified') return { label: 'Lean verified', tone: 'good' };
  const target = [...submission.lean_statements]
    .reverse()
    .find(
      (target) =>
        target.claim === claim &&
        target.version === submission.versions.at(-1)?.number &&
        target.claims_revision === submission.claims_revision,
    );
  if (target) {
    if (target.response.state === 'awaiting_author')
      return { label: 'Awaiting statement confirmation', tone: 'accent' };
    if (target.response.state === 'rejected')
      return { label: 'Statement revision requested', tone: 'accent' };
    if (item?.state.state !== 'in_progress')
      return { label: 'Target confirmed · proof not yet verified', tone: 'muted' };
  }
  switch (item?.state.state) {
    case 'proposed':
      return { label: 'Awaiting your approval', tone: 'accent' };
    case 'approved':
      return { label: 'Queued for target preparation', tone: 'accent' };
    case 'in_progress':
      return { label: 'In progress', tone: 'accent' };
    case 'declined':
      return { label: 'Declined', tone: 'muted' };
    default:
      return { label: 'Not scheduled', tone: 'muted' };
  }
}
