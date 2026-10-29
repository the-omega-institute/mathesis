import type { RefereeFile, Submission } from '../api/types';
import { currentAuthorRound } from '../lib/authorStanding';
import { Markdown, InlineLatex } from './Markdown';
import { DateText } from './ui';
import { formalizationAdviceText } from '../lib/formalizationAdvice';

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
  const formalization = formalizationAdviceText(submission, round);
  const download = () => {
    const blob = new Blob(
      [
        `# ${letter.subject}\n\n${limitations || missingReadings ? '> Some checks could not be completed. The report describes the available findings and remaining uncertainties.\n\n' : ''}${letter.body}\n\n${letter.note}\n\n${formalization}\n`,
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
  const pending = submission.status.state === 'in_review';
  const report = (
    <section
      className="final-review-report"
      aria-label={pending ? 'Review feedback' : 'Final review report'}
    >
      <div className="section-heading">
        <h2>{pending ? 'Review feedback' : 'Review report'}</h2>
        <button type="button" className="button button-quiet button-small" onClick={download}>
          Download report
        </button>
      </div>
      <p className="entry-meta">
        {pending ? 'Feedback prepared' : 'Reviewed'} · <DateText iso={letter.sent_at} />
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
      <div className="formalization-opportunities">
        <Markdown text={formalization} />
      </div>
    </section>
  );
  return pending ? (
    <details className="history pending-feedback">
      <summary>Available feedback — decision pending</summary>
      <p>
        The publication decision is still pending. This feedback is not a final acceptance or
        refusal.
      </p>
      {report}
    </details>
  ) : (
    report
  );
}
