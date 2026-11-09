import { t } from '../i18n/locale';
import { GitHubLink } from '../components/GitHubLink';
import { ProofWork } from '../components/ProofWork';
import { ProofAttribution } from '../components/ProofAttribution';
import { useCallback } from 'react';
import { useApi } from '../api/context';
import { useAsync } from '../api/useAsync';
import { Async } from '../components/ui';
import { InlineLatex, LatexText, MacrosProvider } from '../components/Markdown';
import { ReportError, ErrorReports } from '../components/ErrorReports';
import { AttemptForm } from './Conjecture';
import { EntrantName } from './Leaderboard';
import { Link } from '../routing/router';

export function SupportersPage() {
  return (
    <div className="page narrow-page">
      <header className="page-head">
        <div>
          <h1>{t('Help us build this better')}</h1>
          <p className="lede">
            {t('We welcome collaboration on mathematical research and formalization.')}
          </p>
          <p>
            {t(
              'Help improve mathematical review, formalization and the way research is shared. Ideas, bug reports and code contributions are welcome.',
            )}
          </p>
          <p>
            <GitHubLink label="Contribute on GitHub" />
          </p>
          <p>{t('For research collaborations and other enquiries, please get in touch.')}</p>
          <p>
            <a href="mailto:contact@chrono-ai.fun">contact@chrono-ai.fun</a>
          </p>
        </div>
      </header>
    </div>
  );
}
export function StatementPage({ record, claim }: { record: string; claim: string }) {
  const reviewed = record.startsWith('WP-');
  const api = useApi();
  const load = useCallback(
    (signal: AbortSignal) => api.getStatement(record, claim, { signal }),
    [api, record, claim],
  );
  const statement = useAsync(load);
  return (
    <div className="page">
      <Async state={statement.state} onRetry={statement.reload}>
        {(s) => (
          <MacrosProvider macros={s.macros}>
            <article>
              <p className="eyebrow">
                <Link
                  to={{
                    kind: 'paper',
                    record,
                    version: s.publication ? String(s.publication.version) : undefined,
                  }}
                >
                  <InlineLatex source={s.title} />
                </Link>
              </p>
              <h1>
                <InlineLatex source={s.claim.label} />
              </h1>
              <LatexText source={s.claim.statement} />
              {s.claim.verified ? (
                <p className="notice">
                  <strong>
                    {t(
                      s.claim.verified.receipt.verdict === 'disproved'
                        ? 'Disproved in Lean'
                        : 'Lean ✓',
                    )}
                  </strong>{' '}
                  · <EntrantName entrant={s.claim.verified.entrant} />{' '}
                  {t(
                    s.claim.verified.receipt.verdict === 'disproved'
                      ? 'provided a checked counterexample to this Lean statement.'
                      : reviewed
                        ? 'proved the author-confirmed Lean target.'
                        : 'proved this exact Lean statement.',
                  )}
                </p>
              ) : null}
              <section>
                <h2>{t('Lean target')}</h2>
                {s.target ? (
                  <>
                    <p>
                      {t(
                        reviewed
                          ? 'The author confirmed this exact target.'
                          : 'This Lean statement was checked against the published result.',
                      )}
                    </p>
                    <details>
                      <summary>{t('View Target.lean')}</summary>
                      <pre>
                        <code>{s.target.lean}</code>
                      </pre>
                    </details>
                    <a
                      className="button button-quiet"
                      href={`/api/v1/papers/${encodeURIComponent(record)}/statements/${encodeURIComponent(claim)}/target`}
                      download="Target.lean"
                    >
                      {t('Download Target.lean')}
                    </a>
                    {reviewed ? (
                      <ReportError
                        record={record}
                        subject={{ kind: 'target', claim, digest: s.target.digest }}
                      />
                    ) : null}
                  </>
                ) : (
                  <p className="muted">
                    {t(
                      'A matching Lean statement is being prepared. You can read the mathematical result and contribute existing Lean work in the meantime.',
                    )}{' '}
                    <Link
                      to={{
                        kind: 'result',
                        record,
                        claim,
                        version:
                          !reviewed && s.publication ? String(s.publication.version) : undefined,
                      }}
                    >
                      {t('Read result')}
                    </Link>
                  </p>
                )}
              </section>
              {s.target ? (
                <section>
                  <h2>{t('Contribute a formalization')}</h2>
                  <Link className="button" to={{ kind: 'submit', intake: 'lean', record, claim }}>
                    {t('Submit attributed Lean work')}
                  </Link>
                  <details className="history">
                    <summary>{t('Submit a proof adapter directly')}</summary>
                    <p>
                      {t(
                        'Import Target and prove wishpool_solution : wishpool_target_prop, or wishpool_disproof : ¬ wishpool_target_prop.',
                      )}
                    </p>
                    <AttemptForm record={record} claim={claim} statement />
                  </details>
                </section>
              ) : (
                <Link
                  className="button"
                  to={{
                    kind: 'submit',
                    intake: 'lean',
                    record,
                    claim,
                    version: !reviewed && s.publication ? String(s.publication.version) : undefined,
                  }}
                >
                  {t('Contribute Lean work')}
                </Link>
              )}
              {s.verified_attempts.length ? (
                <section>
                  <h2>{t('Verified attempts')}</h2>
                  <ul className="entry-list">
                    {s.verified_attempts.map((a) => (
                      <li key={a.id}>
                        <EntrantName entrant={a.entrant} />
                        <ProofAttribution attribution={a.attribution} />
                        <a
                          href={`/api/v1/papers/${encodeURIComponent(record)}/statements/${encodeURIComponent(claim)}/attempts/${encodeURIComponent(a.id)}/bundle`}
                        >
                          {t('Download proof and verification details')}
                        </a>
                        <p>
                          {a.receipt.verdict === 'proved' ? t('Proved') : t('Disproved')}{' '}
                          {t(
                            reviewed
                              ? ' the author-confirmed Lean target.'
                              : ' this exact Lean statement.',
                          )}
                        </p>
                        <details>
                          <summary>{t('View Lean proof')}</summary>
                          <pre>{a.solution}</pre>
                          {Object.entries(a.modules ?? {}).map(([name, source]) => (
                            <div key={name}>
                              <h4>
                                {name}
                                {t('.lean')}
                              </h4>
                              <pre>{source}</pre>
                            </div>
                          ))}
                        </details>
                      </li>
                    ))}
                  </ul>
                </section>
              ) : null}
              {s.target ? <ProofWork record={record} claim={claim} /> : null}
              {reviewed ? (
                <details className="history">
                  <summary>{t('Corrections')}</summary>
                  <ReportError record={record} subject={{ kind: 'statement', claim }} />
                  <ErrorReports record={record} />
                </details>
              ) : null}
            </article>
          </MacrosProvider>
        )}
      </Async>
    </div>
  );
}
