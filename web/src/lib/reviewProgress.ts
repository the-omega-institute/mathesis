import type { FeedbackLetter, RefereeFile, RefereeRound, Submission } from '../api/types';

export function hasWaitingResponse(file: RefereeFile | undefined, submission: Submission): boolean {
  const version = submission.versions.at(-1)?.number;
  return (
    file?.finding_responses?.some(
      (response) =>
        response.version === version &&
        response.claims_revision === submission.claims_revision &&
        !file.rounds.some(
          (round) =>
            round.version === version &&
            round.claims_revision === submission.claims_revision &&
            round.response_ids?.includes(response.id),
        ),
    ) ?? false
  );
}

export const REVIEW_STAGES = [
  ['referee', 'Referee report'],
  ['audit', 'Source check'],
  ['advice', 'Improvement suggestions'],
  ['letter', 'Feedback letter'],
  ['formal', 'Lean check'],
] as const;

export function reviewDelivered(round: RefereeRound, letters: readonly FeedbackLetter[]) {
  return letters.some((letter) => letter.round === round.number);
}

/** Only a delivered letter establishes delivery; hidden drafts never do. */
export function reviewProgressText(
  round: RefereeRound,
  letters: readonly FeedbackLetter[],
  includeFormal = false,
) {
  if (!round.progress) return undefined;
  const delivered = reviewDelivered(round, letters);
  const stages = delivered
    ? includeFormal
      ? REVIEW_STAGES.slice(4)
      : []
    : REVIEW_STAGES.slice(0, 4);
  for (const [key, label] of stages) {
    const step = round.progress[key];
    if (step.state === 'failed') return failedStageText(key);
    if (step.state === 'running')
      return `${label} ${key === 'advice' ? 'are' : 'is'} in progress. You can leave this page and return later.`;
    if (step.state === 'pending')
      return `${label} ${key === 'advice' ? 'are' : 'is'} waiting to start. You can leave this page and return later.`;
  }
  return delivered
    ? 'Your feedback letter is ready below.'
    : 'The review stages have finished. No feedback letter has been delivered for this round.';
}

/** Translate a worker failure into the next useful action for an author. */
function failedStageText(key: (typeof REVIEW_STAGES)[number][0]): string {
  const label = REVIEW_STAGES.find(([stage]) => stage === key)![1];
  return `${label} needs attention. Your upload and any report are saved. A failed step is not a verdict on the mathematics. A corrected version may be uploaded.`;
}
