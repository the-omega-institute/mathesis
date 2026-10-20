import { useState, type FormEvent } from 'react';
import type { RefereeFile, RefereeRound, Submission } from '../api/types';
import { useApi } from '../api/context';
import { useSession } from '../auth/session';
import { InlineLatex, Markdown } from './Markdown';
import { DateText, Field, InlineError } from './ui';
import { authorReview, deliveredLetter } from '../lib/authorReview';

export function ReviewFindings({
  submission,
  file,
  onChange,
  presentation = 'staff',
  reportRound,
}: {
  submission: Submission;
  file: RefereeFile;
  onChange: (file: RefereeFile) => void;
  presentation?: 'staff' | 'author';
  reportRound?: number;
}) {
  const { person } = useSession();
  const author =
    !!person &&
    (submission.submitter === person.id || submission.authors.some((a) => a.person === person.id));
  const deliveredRound = reportRound ?? authorReview(submission, file).reportRound?.number;
  const rounds = [...file.rounds]
    .reverse()
    .filter((r) => presentation === 'staff' || r.number === deliveredRound)
    .filter(
      (r) =>
        r.audit?.state.state === 'done' &&
        r.audit.state.result.concerns.some(
          (finding) => presentation === 'staff' || finding.status !== 'refuted',
        ),
    );
  if (!rounds.length) return null;
  return (
    <section
      className="review-findings"
      aria-label={presentation === 'author' ? 'Points to address' : 'Review findings and responses'}
    >
      <h3>{presentation === 'author' ? 'Points to address' : 'Findings to address'}</h3>
      <p className="small muted">
        {presentation === 'author'
          ? 'These are the points the review could not settle. Open one to read the detail, explain a correction, or request another review.'
          : 'These are the points the source check could not settle. Open one to read the evidence, explain a correction, or upload a revised version and request another review. A response does not mark a finding resolved.'}
      </p>
      {rounds.map((round) => (
        <div key={round.number}>
          {presentation === 'staff' ? (
            <h4>
              Round {round.number} · source version {round.version}
            </h4>
          ) : null}
          {presentation === 'staff' && round.source_sha256 ? (
            <p className="hint">
              Source SHA-256:{' '}
              <code style={{ overflowWrap: 'anywhere' }}>{round.source_sha256}</code>
            </p>
          ) : null}
          {round.audit?.state.state === 'done' &&
            round.audit.state.result.concerns.map((finding, index) => {
              if (presentation === 'author' && finding.status === 'refuted') return null;
              const responses = (file.finding_responses ?? []).filter(
                (r) => r.round === round.number && r.concern === index,
              );
              return (
                <details className="finding-card" key={index}>
                  <summary>
                    {presentation === 'staff' ? (
                      <strong>
                        R{round.number}-A{index + 1}
                      </strong>
                    ) : null}{' '}
                    {presentation === 'staff' ? `· ${finding.status.replace('_', ' ')} · ` : ''}
                    <InlineLatex source={finding.concern} />
                  </summary>
                  {finding.location ? (
                    <p className="small">Reported location: {finding.location}</p>
                  ) : null}
                  <Markdown text={finding.note} />
                  {finding.evidence ? (
                    <div className="finding-evidence">
                      <h5>{presentation === 'author' ? 'Review detail' : 'Supporting evidence'}</h5>
                      {presentation === 'staff' ? (
                        <p className="hint">
                          {finding.basis === 'opened_source'
                            ? 'The reviewer reports opening the cited source; the platform has not independently authenticated it.'
                            : 'Review evidence; this is not a Lean verification receipt.'}
                        </p>
                      ) : null}
                      <Markdown text={finding.evidence} />
                    </div>
                  ) : (
                    <p className="hint">
                      {presentation === 'author'
                        ? 'No further detail was provided for this point.'
                        : 'No separate evidence was recorded for this finding.'}
                    </p>
                  )}
                  {responses.map((response) => {
                    const checked = [...file.rounds]
                      .reverse()
                      .find(
                        (r) =>
                          r.version === response.version &&
                          r.claims_revision === response.claims_revision &&
                          r.response_ids?.includes(response.id) &&
                          (presentation === 'staff' || Boolean(deliveredLetter(r, file.letters))) &&
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
                          Your response · <DateText iso={response.at} />
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
                              </strong>
                            </p>
                            <Markdown text={assessment.explanation} />
                            {assessment.evidence.length ? (
                              <ul>
                                {assessment.evidence.map((e) => (
                                  <li key={e.path}>
                                    <a
                                      href={`/api/v1/submissions/${encodeURIComponent(submission.id)}/evidence?version=${response.version}&path=${encodeURIComponent(e.path)}`}
                                    >
                                      {e.path}
                                    </a>
                                    {presentation === 'staff' ? (
                                      <p className="hint">
                                        SHA-256:{' '}
                                        <code style={{ overflowWrap: 'anywhere' }}>{e.sha256}</code>
                                      </p>
                                    ) : null}
                                  </li>
                                ))}
                              </ul>
                            ) : null}
                            {presentation === 'staff' ? (
                              <p className="hint">
                                Reviewer conclusion on this source version; not an independent proof
                                or execution receipt.
                              </p>
                            ) : null}
                          </div>
                        ) : null}
                        <p className="hint">
                          {!current
                            ? presentation === 'author'
                              ? 'This response belongs to an earlier version of the paper.'
                              : 'This response belongs to an earlier source or statement revision.'
                            : checked && !assessment
                              ? presentation === 'author'
                                ? 'This response is being reviewed.'
                                : `Re-reviewed in round ${checked.number}. Read that audit for the conclusion.`
                              : !checked
                                ? presentation === 'author'
                                  ? 'A follow-up review has been requested.'
                                  : 'Re-review requested. A new audit will address this response.'
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
                      presentation={presentation}
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
  presentation,
  onChange,
}: {
  submission: Submission;
  round: RefereeRound;
  concern: number;
  presentation: 'staff' | 'author';
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
      aria-label={
        presentation === 'author'
          ? 'Respond to this point'
          : `Respond to R${round.number}-A${concern + 1}`
      }
    >
      <Field
        label="Your response or correction"
        htmlFor={label}
        hint={
          presentation === 'author'
            ? 'Upload and confirm a revision first if the source has changed.'
            : `This requests review of source version ${submission.versions.at(-1)?.number}. Upload and confirm a revision first if the source has changed.`
        }
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
