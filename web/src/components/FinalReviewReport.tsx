import type { RefereeFile, Submission } from '../api/types';
import { currentAuthorRound } from '../lib/authorStanding';
import { authorReport, authorReportPreview } from '../lib/authorReport';
import { Markdown } from './Markdown';
import { DateText } from './ui';

export function currentReport(submission: Submission, file: RefereeFile) {
  const round = currentAuthorRound(submission, file);
  return round
    ? [...file.letters]
        .filter((letter) => letter.round === round.number)
        .sort((a, b) => Date.parse(b.sent_at) - Date.parse(a.sent_at))[0]
    : undefined;
}

export function FinalReviewReport({
  submission,
  file,
}: {
  submission: Submission;
  file: RefereeFile;
}) {
  const round = currentAuthorRound(submission, file);
  const letter = currentReport(submission, file);
  if (['draft', 'in_review', 'withdrawn'].includes(submission.status.state)) return null;
  if (!round || !letter)
    return <p className="muted">The final report will appear here when the review is complete.</p>;
  const text = authorReport(submission, round, letter);
  const preview = authorReportPreview(submission, round, letter);
  if (!text || !preview) return null;
  const download = () => {
    const blob = new Blob(
      [`# Mathesis review: ${submission.title}\n\n${preview}\n\n## Detailed report\n\n${text}\n`],
      {
        type: 'text/markdown;charset=utf-8',
      },
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
        Reviewed · <DateText iso={letter.sent_at} />
      </p>
      <Markdown text={preview} texDelimiters />
      <details className="history report-details">
        <summary>Detailed report: findings and recommendations</summary>
        <Markdown text={text} texDelimiters />
      </details>
    </section>
  );
}
