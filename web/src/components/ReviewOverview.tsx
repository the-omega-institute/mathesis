import type { FeedbackLetter, RefereeRound, Submission } from '../api/types';
import { Badge, type BadgeTone } from './ui';
import { formalizationStatus } from '../lib/formalizationStatus';

type ReviewOverviewProps = {
  submission: Submission;
  round?: RefereeRound;
  letters?: readonly FeedbackLetter[];
};

type ReviewState = { label: string; tone: BadgeTone };

/** A compact staff-facing answer to "what has completed?". */
export function ReviewOverview({ submission, round, letters = [] }: ReviewOverviewProps) {
  const currentLetters = round ? letters.filter((letter) => letter.round === round.number) : [];
  return (
    <section className="review-overview" aria-label="Review overview">
      <p className="eyebrow">Review overview</p>
      <h2 id="review-overview-h">What is complete</h2>
      <p className="review-overview-summary" role="status">
        {nextAction(submission, round, currentLetters.length > 0)}
      </p>
      <dl className="review-overview-grid">
        <OverviewItem label="Referee report" state={stepState(round?.referee, 'Complete')} />
        <OverviewItem label="Source check" state={stepState(round?.audit, 'Complete')} />
        <OverviewItem
          label="Author feedback"
          state={
            currentLetters.length > 0
              ? { label: 'Delivered', tone: 'good' }
              : stepState(round?.letter, 'Ready to send')
          }
        />
        <OverviewItem label="Publication" state={publicationState(submission)} />
        <OverviewItem label="Formalization" state={formalizationState(submission)} />
      </dl>
      {round ? (
        <p className="small muted review-overview-note">
          Current review round {round.number} · source version {round.version}. Execution history
          records runs and retries; only saved results count as completed review work.
        </p>
      ) : (
        <p className="small muted review-overview-note">No review round has been started.</p>
      )}
    </section>
  );
}

function OverviewItem({ label, state }: { label: string; state: ReviewState }) {
  return (
    <div>
      <dt>{label}</dt>
      <dd>
        <Badge tone={state.tone}>{state.label}</Badge>
      </dd>
    </div>
  );
}

function stepState(
  step: RefereeRound['referee'] | RefereeRound['audit'] | RefereeRound['letter'] | undefined,
  completeLabel: string,
): ReviewState {
  if (!step) return { label: 'Not started', tone: 'muted' };
  switch (step.state.state) {
    case 'done':
      return { label: completeLabel, tone: 'good' };
    case 'failed':
      return { label: 'Needs attention', tone: 'bad' };
    case 'running':
      return { label: 'In progress', tone: 'accent' };
    case 'pending':
      return { label: 'Waiting', tone: 'muted' };
    case 'skipped':
      return { label: 'Skipped', tone: 'muted' };
  }
}

function publicationState(submission: Submission): ReviewState {
  if (submission.purpose === 'review_only') return { label: 'Private review', tone: 'muted' };
  switch (submission.status.state) {
    case 'accepted':
      return { label: 'Accepted', tone: 'good' };
    case 'not_accepted':
      return { label: 'Not accepted', tone: 'bad' };
    case 'review_complete':
      return { label: 'No decision recorded', tone: 'muted' };
    case 'in_review':
      return { label: 'In progress', tone: 'accent' };
    case 'draft':
      return { label: 'Not started', tone: 'muted' };
    case 'withdrawn':
      return { label: 'Withdrawn', tone: 'muted' };
  }
}

function formalizationState(submission: Submission): ReviewState {
  if (submission.purpose === 'review_only') return { label: 'Not scheduled', tone: 'muted' };
  if (submission.status.state !== 'accepted') return { label: 'After acceptance', tone: 'muted' };
  const claims = new Set([
    ...submission.formalization.items.map((item) => item.claim),
    ...submission.lean_statements
      .filter(
        (target) =>
          target.version === submission.versions.at(-1)?.number &&
          target.claims_revision === submission.claims_revision,
      )
      .map((target) => target.claim),
  ]);
  if (!claims.size)
    return {
      label: submission.kind === 'conjecture' ? 'Target not yet available' : 'Not scheduled',
      tone: 'muted',
    };
  const counts = new Map<string, number>();
  let active = false;
  let verified = false;
  for (const claim of claims) {
    const status = formalizationStatus(submission, claim);
    const label = status.label.replace('Awaiting your approval', 'Awaiting author approval');
    counts.set(label, (counts.get(label) ?? 0) + 1);
    active ||= status.tone === 'accent';
    verified ||= status.tone === 'good';
  }
  return {
    label: [...counts].map(([label, count]) => `${count} ${label.toLowerCase()}`).join(' · '),
    tone: active ? 'accent' : verified ? 'good' : 'muted',
  };
}

function nextAction(
  submission: Submission,
  round: RefereeRound | undefined,
  delivered: boolean,
): string {
  if (!round) return 'No review result has been recorded yet.';
  if (delivered)
    return 'The final report has been delivered. Formalization progress is tracked separately from the review.';
  const steps = [
    ['referee', 'Manuscript assessment'],
    ['audit', 'Source check'],
    ['advice', 'Suggestions'],
    ['letter', 'Final report'],
  ] as const;
  for (const [key, label] of steps) {
    const state = round[key]?.state.state ?? round.progress?.[key].state;
    if (state === 'failed')
      return `${label} did not finish. The final report has not been delivered. Inspect the execution history before retrying.`;
    if (state === 'running')
      return `${label} is in progress. The final report has not been delivered.`;
    if (state === 'pending')
      return `${label} is waiting to start. The final report has not been delivered.`;
  }
  if (submission.status.state === 'accepted')
    return 'Acceptance is recorded. The final report has not been delivered yet.';
  return 'Review results are saved. The final report has not been delivered yet.';
}
