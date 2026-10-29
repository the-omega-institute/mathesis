import type { RefereeFile, Submission } from '../api/types';
import { currentAuthorRound } from '../lib/authorStanding';
import { Markdown, InlineLatex } from './Markdown';
import { DateText } from './ui';

export function currentReport(submission: Submission, file: RefereeFile) {
  const round = currentAuthorRound(submission, file);
  return round
    ? [...file.letters]
        .filter((letter) => letter.round === round.number)
        .sort((a, b) => Date.parse(b.sent_at) - Date.parse(a.sent_at))[0]
    : undefined;
}

/** Publication decisions and delivered reports are separate from internal recommendations. */
export function FinalReviewReport({
  submission,
  file,
}: {
  submission: Submission;
  file: RefereeFile;
}) {
  const round = currentAuthorRound(submission, file);
  const letter = currentReport(submission, file);
  if (!letter)
    return <p className="muted">The final report will appear here when the review is complete.</p>;
  const limitations =
    round &&
    (['referee', 'audit', 'advice', 'letter'] as const).some(
      (key) => (round.progress?.[key] ?? round[key]?.state)?.state === 'failed',
    );
  const missingReadings =
    round?.referee.state.state === 'done' && !round.referee.state.result.claims.length;
  const download = () => {
    const blob = new Blob(
      [
        `# ${letter.subject}\n\n${limitations || missingReadings ? '> Some checks could not be completed. The report describes the available findings and remaining uncertainties.\n\n' : ''}${letter.body}\n\n${letter.note}\n`,
      ],
      { type: 'text/markdown;charset=utf-8' },
    );
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.download = 'mathesis-review.md';
    link.click();
    window.setTimeout(() => URL.revokeObjectURL(url), 1000);
  };
  return (
    <section className="final-review-report" aria-label="Final review report">
      <div className="section-heading">
        <h2>Review report</h2>
        <button type="button" className="button button-quiet button-small" onClick={download}>
          Download report
        </button>
      </div>
      <p className="entry-meta">
        AI review completed · <DateText iso={letter.sent_at} />
      </p>
      {limitations || missingReadings ? (
        <p className="notice notice-info">
          Some checks could not be completed. The report describes the available findings and
          remaining uncertainties.
        </p>
      ) : null}
      <h3>
        <InlineLatex source={letter.subject} />
      </h3>
      <Markdown text={letter.body} lineBreaks />
      {letter.note ? (
        <div className="review-analysis">
          <Markdown text={letter.note} />
        </div>
      ) : null}
      <p className="small">
        {submission.purpose === 'review_only'
          ? 'No formalization is scheduled for this private review.'
          : submission.status.state === 'not_accepted'
            ? 'Formalization is not scheduled for this submission.'
            : submission.kind === 'conjecture'
              ? 'Community members can contribute a proof once a Lean target is available.'
              : round?.formal?.state.state === 'done'
                ? 'Formalization results are available in the manuscript details.'
                : round?.formal?.state.state === 'running'
                  ? 'A formalization attempt is in progress.'
                  : 'Any formalization plan will be shown separately when it is available.'}
      </p>
    </section>
  );
}
