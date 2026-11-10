import type { RefereeFile, RefereeRound, Submission } from '../api/types';
import { authorReview } from '../lib/authorReview';
import { formalizationStatus } from '../lib/formalizationStatus';
import { CLAIM_KIND_LABELS, isOpenKind } from '../lib/labels';
import { ProofAttribution } from './ProofAttribution';
import { ReviewFindings } from './ReviewFindings';
import { LatexText, Markdown } from './Markdown';
import { Badge, DateText } from './ui';
import { LeanBadge } from './badges';
import { publicationReason } from './PaperStanding';
import { statementName } from './StatementSummary';

type FinalReviewReportProps = {
  submission: Submission;
  file: RefereeFile;
  showStatus?: boolean;
  onChange: (file: RefereeFile) => void;
  onRevise?: () => void;
};

/**
 * The author-facing review surface. Worker stages are deliberately collapsed
 * into one report: a decision, the reasons for it, and the statements that
 * continue into formalization.
 */
export function FinalReviewReport({
  submission,
  file,
  showStatus = true,
  onChange,
  onRevise,
}: FinalReviewReportProps) {
  const { current, reportRound, letter, status } = authorReview(submission, file);
  const accepted = submission.status.state === 'accepted';
  const canRevise =
    Boolean(onRevise) &&
    ['accepted', 'not_accepted', 'review_complete'].includes(submission.status.state);
  const earlier = file.letters.filter((item) => {
    if (item.round === undefined) return true;
    const origin = file.rounds.find((r) => r.number === item.round);
    return (
      origin &&
      origin.number !== reportRound?.number &&
      (origin.version !== submission.versions.at(-1)?.number ||
        origin.claims_revision !== submission.claims_revision ||
        (reportRound && origin.number < reportRound.number))
    );
  });

  return (
    <section className="referee-panel final-review-report" aria-label="AI review report">
      <div className="final-report-heading">
        <div>
          <p className="eyebrow">AI review</p>
          <h2 id="final-review-h">Review report</h2>
        </div>
        <Badge tone={status.tone}>{status.label}</Badge>
      </div>
      {showStatus ? (
        <p className="final-report-status" role="status">
          {status.description}
        </p>
      ) : null}

      {letter ? (
        <section className="final-report-section final-report-body" aria-labelledby="report-text-h">
          <h3 id="report-text-h">
            {reportRound?.number !== current?.number
              ? 'Previously delivered report'
              : 'Final report'}
          </h3>
          <p className="small muted">
            Version {reportRound?.version} · Delivered <DateText iso={letter.sent_at} />
          </p>
          <Markdown text={letter.body} lineBreaks />
          {letter.note.trim() ? (
            <div className="final-report-note">
              <p className="small">
                <strong>Additional detail</strong>
              </p>
              <Markdown text={letter.note} />
            </div>
          ) : null}
        </section>
      ) : null}

      {submission.status.state === 'not_accepted' &&
      submission.decision?.decision === 'not_accepted' ? (
        <section className="final-report-section" aria-labelledby="decision-reasons-h">
          <h3 id="decision-reasons-h">Why this decision was reached</h3>
          <ul className="final-report-reasons">
            {submission.decision.reasons.map((reason, index) => (
              <li key={index}>{publicationReason(reason)}</li>
            ))}
          </ul>
        </section>
      ) : null}

      {accepted && reportRound ? (
        <AcceptedStatements submission={submission} round={reportRound} />
      ) : null}

      {letter && reportRound ? (
        <ReviewFindings
          submission={submission}
          file={file}
          onChange={onChange}
          presentation="author"
          reportRound={reportRound.number}
        />
      ) : null}

      {canRevise ? (
        <div className="final-report-next">
          <p>
            {accepted
              ? 'You may upload a revised version if the report requests changes.'
              : 'You can revise the paper and submit a new version.'}
          </p>
          <button type="button" className="button button-small" onClick={onRevise}>
            Upload a revised version
          </button>
        </div>
      ) : null}

      {letter ? (
        <div className="final-report-downloads" aria-label="Report files">
          <a
            className="button button-quiet"
            href={`/api/v1/submissions/${encodeURIComponent(submission.id)}/review.md`}
          >
            Download report
          </a>
          <details>
            <summary>Data export</summary>
            <a href={`/api/v1/submissions/${encodeURIComponent(submission.id)}/review`}>
              Structured review data (JSON)
            </a>
          </details>
        </div>
      ) : null}
      {earlier.length ? (
        <details className="history">
          <summary>Earlier reports</summary>
          {earlier.map((item, index) => (
            <section key={index} className="final-report-section">
              <h3>
                {item.round === undefined
                  ? 'Earlier report · version not recorded'
                  : `Report on version ${file.rounds.find((r) => r.number === item.round)?.version}`}
              </h3>
              <p className="small muted">
                Delivered <DateText iso={item.sent_at} />
              </p>
              <Markdown text={item.body} lineBreaks />
              {item.note.trim() ? <Markdown text={item.note} /> : null}
            </section>
          ))}
        </details>
      ) : null}
    </section>
  );
}

function AcceptedStatements({
  submission,
  round,
}: {
  submission: Submission;
  round?: RefereeRound;
}) {
  const audit = round?.audit?.state.state === 'done' ? round.audit.state.result : undefined;
  const main = submission.claims.filter((claim) => claim.role === 'main');
  const supporting = submission.claims.filter((claim) => claim.role !== 'main');
  const accepted = (claim: (typeof submission.claims)[number]) => {
    if (isOpenKind(claim.kind)) return false;
    const reading = audit?.claims.find((item) => item.claim === claim.id);
    return reading?.correctness === 'correct' && !reading.known && reading.shape === 'content';
  };
  const renderClaim = (claim: (typeof submission.claims)[number]) => {
    const reading = audit?.claims.find((item) => item.claim === claim.id);
    const formal = submission.formalization.items.find((item) => item.claim === claim.id);
    const verified = formal?.state.state === 'verified' ? formal.state : undefined;
    const title = statementName(claim);
    return (
      <li className="accepted-statement" key={claim.id}>
        <div className="accepted-statement-head">
          <div>
            <p className="eyebrow">
              {CLAIM_KIND_LABELS[claim.kind]} · {claim.id}
            </p>
            <h4>{title ?? claim.label}</h4>
          </div>
          <div className="statement-tags">
            {reading?.shape === 'content' ? <span className="tag tag-accent">New</span> : null}
            {verified ? <span className="tag tag-good">Lean verified</span> : null}
          </div>
        </div>
        <LatexText source={claim.statement} className="statement" />
        <p className="small muted">
          <strong>Authors:</strong>{' '}
          {submission.authors.map((author) => author.name).join(', ') || 'Not listed'}
        </p>
        {verified ? (
          <div className="formal-credit">
            <p className="small">
              <strong>Formalization</strong> · verified in Lean
            </p>
            <LeanBadge artifact={verified.artifact} />
            {verified.attribution ? <ProofAttribution attribution={verified.attribution} /> : null}
            {verified.contribution ? (
              <p className="small muted">Contribution: {verified.contribution}</p>
            ) : null}
          </div>
        ) : (
          <p className="small muted">
            Formalization: {formalizationStatus(submission, claim.id).label}.
          </p>
        )}
      </li>
    );
  };
  const mainAccepted = main.filter(accepted);
  const supportingAccepted = supporting.filter(accepted);
  if (mainAccepted.length === 0 && supportingAccepted.length === 0) return null;
  return (
    <section
      className="final-report-section accepted-statements"
      aria-labelledby="accepted-statements-h"
    >
      <h3 id="accepted-statements-h">Accepted statements</h3>
      <p className="small muted">
        Results identified as new mathematical content in the completed review. Formalization status
        is shown separately for each result.
      </p>
      {mainAccepted.length > 0 ? (
        <>
          <h4 className="accepted-group-title">Main result{mainAccepted.length > 1 ? 's' : ''}</h4>
          <ul className="accepted-statement-list">{mainAccepted.map(renderClaim)}</ul>
        </>
      ) : null}
      {supportingAccepted.length > 0 ? (
        <>
          <h4 className="accepted-group-title">Supporting results</h4>
          <ul className="accepted-statement-list">{supportingAccepted.map(renderClaim)}</ul>
        </>
      ) : null}
    </section>
  );
}
