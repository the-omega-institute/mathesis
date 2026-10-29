import { useState, type FormEvent } from 'react';
import type { RefereeFile, RefereeRound, Submission } from '../api/types';
import { useApi } from '../api/context';
import { useSession } from '../auth/session';
import { manuscriptReferences } from '../lib/statements';
import { AUDIT_FINDING_LABELS } from '../lib/labels';
import { InlineLatex, Markdown } from './Markdown';
import { DateText, Field, InlineError } from './ui';

export function ReviewFindings({
  submission,
  file,
  onChange,
  visibleRound,
  excludeRound,
  showSourceIdentity = true,
}: {
  submission: Submission;
  file: RefereeFile;
  visibleRound?: number;
  excludeRound?: number;
  showSourceIdentity?: boolean;
  onChange: (file: RefereeFile) => void;
}) {
  const readable = (text: string) =>
    showSourceIdentity ? text : manuscriptReferences(text, submission.claims);
  const { person } = useSession();
  const author =
    !!person &&
    (submission.submitter === person.id || submission.authors.some((a) => a.person === person.id));
  const rounds = [...file.rounds]
    .reverse()
    .filter(
      (r) =>
        r.audit?.state.state === 'done' &&
        r.audit.state.result.concerns.length &&
        (visibleRound === undefined || r.number === visibleRound) &&
        r.number !== excludeRound,
    );
  if (!rounds.length) return null;
  return (
    <section className="review-findings" aria-label="Review findings and responses">
      <h3>Review findings</h3>
      <p className="small muted">
        Read the findings below and respond with a clarification or correction. Your response will
        be considered in a follow-up review.
      </p>
      {rounds.map((round) => (
        <div key={round.number}>
          <h4>
            {showSourceIdentity
              ? `Round ${round.number} · source version ${round.version}`
              : `Review of version ${round.version}`}
          </h4>
          {showSourceIdentity && round.source_sha256 ? (
            <p className="hint">
              Source SHA-256:{' '}
              <code style={{ overflowWrap: 'anywhere' }}>{round.source_sha256}</code>
            </p>
          ) : null}
          {round.audit?.state.state === 'done' &&
            round.audit.state.result.concerns.map((finding, index) => {
              const responses = (file.finding_responses ?? []).filter(
                (r) => r.round === round.number && r.concern === index,
              );
              return (
                <details className="finding-card" key={index}>
                  <summary>
                    <strong>
                      {showSourceIdentity
                        ? `R${round.number}-A${index + 1}`
                        : `Finding ${index + 1}`}
                    </strong>{' '}
                    · {AUDIT_FINDING_LABELS[finding.status]} ·{' '}
                    <InlineLatex source={readable(finding.concern)} />
                  </summary>
                  {finding.location ? (
                    <p className="small">Reported location: {finding.location}</p>
                  ) : null}
                  <Markdown text={readable(finding.note)} />
                  {finding.evidence ? (
                    <div className="finding-evidence">
                      <h5>Supporting evidence</h5>
                      <p className="hint">
                        {finding.basis === 'opened_source'
                          ? 'The reviewer reports opening the cited source; the citation has not been independently confirmed.'
                          : 'Supporting reasoning from the review.'}
                      </p>
                      <Markdown text={readable(finding.evidence)} />
                    </div>
                  ) : (
                    <p className="hint">No separate evidence was recorded for this finding.</p>
                  )}
                  {responses.map((response) => {
                    const checked = [...file.rounds]
                      .reverse()
                      .find(
                        (r) =>
                          r.version === response.version &&
                          r.claims_revision === response.claims_revision &&
                          r.response_ids?.includes(response.id) &&
                          r.audit?.state.state === 'done',
                      );
                    const current =
                      submission.versions.at(-1)?.number === response.version &&
                      submission.claims_revision === response.claims_revision;
                    const assessment =
                      checked?.audit?.state.state === 'done'
                        ? checked.audit.state.result.response_assessments?.find(
                            (a) => a.response_id === response.id,
                          )
                        : undefined;
                    return (
                      <div className="finding-response" key={response.id}>
                        <p className="small">
                          Author response · version {response.version} ·{' '}
                          <DateText iso={response.at} />
                        </p>
                        <Markdown text={response.body} />
                        {assessment ? (
                          <div className="finding-evidence">
                            <p className="small">
                              <strong>
                                {assessment.status === 'addressed'
                                  ? 'Addressed in this review'
                                  : assessment.status === 'unresolved'
                                    ? 'Concern remains'
                                    : 'Further evidence needed'}
                              </strong>{' '}
                              · round {checked!.number} · version {response.version}
                            </p>
                            <Markdown text={readable(assessment.explanation)} />
                            {assessment.evidence.length ? (
                              <ul>
                                {assessment.evidence.map((e) => (
                                  <li key={e.path}>
                                    <a
                                      href={`/api/v1/submissions/${encodeURIComponent(submission.id)}/evidence?version=${response.version}&path=${encodeURIComponent(e.path)}`}
                                    >
                                      {e.path}
                                    </a>
                                    {showSourceIdentity ? (
                                      <p className="hint">
                                        SHA-256:{' '}
                                        <code style={{ overflowWrap: 'anywhere' }}>{e.sha256}</code>
                                      </p>
                                    ) : null}
                                  </li>
                                ))}
                              </ul>
                            ) : null}
                            <p className="hint">
                              This assessment applies to the manuscript version reviewed.
                            </p>
                          </div>
                        ) : null}
                        <p className="hint">
                          {!current
                            ? 'This response refers to an earlier version of the manuscript.'
                            : checked && !assessment
                              ? `Re-reviewed in round ${checked.number}. Read that report for the conclusion.`
                              : !checked
                                ? 'Re-review requested. A new review will address this response.'
                                : null}
                        </p>
                      </div>
                    );
                  })}
                  {author &&
                  submission.status.state !== 'draft' &&
                  submission.status.state !== 'withdrawn' ? (
                    <ResponseForm
                      submission={submission}
                      round={round}
                      concern={index}
                      onChange={onChange}
                    />
                  ) : null}
                </details>
              );
            })}
        </div>
      ))}
    </section>
  );
}
function ResponseForm({
  submission,
  round,
  concern,
  onChange,
}: {
  submission: Submission;
  round: RefereeRound;
  concern: number;
  onChange: (file: RefereeFile) => void;
}) {
  const api = useApi();
  const [body, setBody] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<unknown>(null);
  const label = `response-${round.number}-${concern}`;
  const submit = async (event: FormEvent) => {
    event.preventDefault();
    setBusy(true);
    setError(null);
    try {
      onChange(
        await api.respondToFinding(submission.id, {
          round: round.number,
          concern,
          version: submission.versions.at(-1)!.number,
          claims_revision: submission.claims_revision,
          body,
        }),
      );
      setBody('');
    } catch (e) {
      setError(e);
    } finally {
      setBusy(false);
    }
  };
  return (
    <form
      className="form"
      onSubmit={(e) => {
        void submit(e);
      }}
      aria-label={`Respond to R${round.number}-A${concern + 1}`}
    >
      <Field
        label="Your response or correction"
        htmlFor={label}
        hint={`This requests review of source version ${submission.versions.at(-1)?.number}. Upload and confirm a revision first if the source has changed.`}
      >
        <textarea
          id={label}
          required
          maxLength={10000}
          value={body}
          onChange={(e) => setBody(e.target.value)}
          rows={3}
        />
      </Field>
      <button className="button button-small" disabled={busy || !body.trim()}>
        {busy ? 'Requesting…' : 'Send response & request re-review'}
      </button>
      <InlineError error={error} />
    </form>
  );
}
