import { useEffect, useState } from 'react';
import type { RefereeFile, RejectReason, Submission } from '../api/types';
import { authorStanding } from '../lib/authorStanding';
import { rejectReasonText } from '../lib/labels';

export function publicationReason(reason: RejectReason): string {
  switch (reason.reason) {
    case 'known_result':
      return `Statement ${reason.claim} already follows from ${reason.prior.locator}.`;
    case 'bind_only':
      return 'No checked main result carries new mathematical content.';
    case 'no_main_result':
      return 'The paper has no proved main result.';
    case 'incorrect_result':
    case 'unchecked_result':
      return rejectReasonText(reason);
    default:
      return reason.detail;
  }
}

export function PaperStanding({
  submission,
  file,
  onRevise,
  showExecutionDetails = false,
}: {
  submission: Submission;
  file?: RefereeFile;
  onRevise?: () => void;
  showExecutionDetails?: boolean;
}) {
  const [now, setNow] = useState(Date.now);
  useEffect(() => {
    if (!showExecutionDetails) return;
    const timer = window.setInterval(() => setNow(Date.now()), 60_000);
    return () => window.clearInterval(timer);
  }, [showExecutionDetails]);
  const standing = authorStanding(submission, file, now);
  const status = submission.status;
  const compileFailure = status.state === 'draft' && !!submission.versions.at(-1)?.compile_error;
  const mayRevise =
    compileFailure || ['accepted', 'not_accepted', 'review_complete'].includes(status.state);
  const tone =
    status.state === 'accepted'
      ? 'good'
      : status.state === 'not_accepted'
        ? 'bad'
        : status.state === 'withdrawn'
          ? 'muted'
          : 'accent';
  const authorMessage =
    status.state === 'draft'
      ? compileFailure
        ? 'The PDF could not be prepared. Upload corrected files to continue.'
        : !submission.versions.at(-1)?.pdf
          ? 'Your manuscript PDF is being prepared.'
          : 'Check the manuscript PDF, then submit for review.'
      : status.state === 'withdrawn'
        ? 'This submission has been withdrawn.'
        : standing.headline.startsWith('Your response')
          ? 'Your response is saved. A follow-up review will address it.'
          : standing.headline === 'Review could not finish'
            ? 'Your manuscript is saved. The review needs attention before it can continue.'
            : standing.delivered
              ? status.state === 'in_review'
                ? 'The publication decision is pending. Available feedback is below.'
                : 'Your review report is ready below.'
              : status.state === 'in_review'
                ? 'Your manuscript is under AI review. The final report will appear here when the review is complete.'
                : !standing.round
                  ? 'Review details are currently unavailable. Your manuscript is saved.'
                  : 'Your final review report is being prepared.';
  return (
    <section className={`standing standing-${tone}`} aria-label="Paper standing">
      <div className="standing-head">
        <h2 className="standing-title">{standing.headline}</h2>
        {status.state === 'accepted' ? (
          <span className="standing-record">{status.record}</span>
        ) : null}
      </div>
      {submission.decision?.decision === 'accept' ? (
        <p>
          {submission.decision.basis === 'escape_witness'
            ? 'The review found new mathematical content in a main result.'
            : submission.decision.basis === 'open_problem_settlement'
              ? 'The review found that a main result settles a named open problem.'
              : 'The review found a well-posed open conjecture whose resolution would carry new mathematical content.'}
        </p>
      ) : null}
      {status.state === 'not_accepted' && submission.decision?.decision === 'not_accepted' ? (
        <ul className="standing-reasons">
          {submission.decision.reasons.map((reason, i) => (
            <li key={i}>{publicationReason(reason)}</li>
          ))}
        </ul>
      ) : null}
      <p role="status">{showExecutionDetails ? standing.doing : authorMessage}</p>
      {showExecutionDetails ? (
        <>
          <p className="standing-next">{standing.nextAction}</p>
          <p className="small">{standing.timing}</p>
        </>
      ) : null}
      {mayRevise && onRevise ? (
        <div className="standing-next">
          <button
            type="button"
            className="button button-small"
            aria-controls="standing-revision-form"
            onClick={onRevise}
          >
            {compileFailure ? 'Upload a corrected version' : 'Upload a revised version'}
          </button>
        </div>
      ) : null}
    </section>
  );
}
