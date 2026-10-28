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
}: {
  submission: Submission;
  file?: RefereeFile;
  onRevise?: () => void;
}) {
  const [now, setNow] = useState(Date.now);
  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now()), 60_000);
    return () => window.clearInterval(timer);
  }, []);
  const standing = authorStanding(submission, file, now);
  const status = submission.status;
  const compileFailure = status.state === 'draft' && !!submission.versions.at(-1)?.compile_error;
  const mayRevise =
    compileFailure ||
    ['in_review', 'accepted', 'not_accepted', 'review_complete'].includes(status.state);
  const tone =
    status.state === 'accepted'
      ? 'good'
      : status.state === 'not_accepted'
        ? 'bad'
        : status.state === 'withdrawn'
          ? 'muted'
          : 'accent';
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
            ? 'The source check found new mathematical content in a main result, without an identified prior result that states or directly implies it.'
            : submission.decision.basis === 'open_problem_settlement'
              ? 'The source check found that a main result settles a named open problem.'
              : 'The source check found a well-posed open conjecture whose resolution would carry new mathematical content.'}
        </p>
      ) : null}
      {status.state === 'not_accepted' && submission.decision?.decision === 'not_accepted' ? (
        <ul className="standing-reasons">
          {submission.decision.reasons.map((reason, i) => (
            <li key={i}>{publicationReason(reason)}</li>
          ))}
        </ul>
      ) : null}
      <p role="status">{standing.doing}</p>
      <p className="standing-next">{standing.nextAction}</p>
      <p className="small">{standing.timing}</p>
      {mayRevise && onRevise ? (
        <div className="standing-next">
          <p>You can revise the paper and submit a new version.</p>
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
