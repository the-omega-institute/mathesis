import { t } from '../i18n/locale';
import { ProofWork } from '../components/ProofWork';
import { ProofAttribution } from '../components/ProofAttribution';
import { useCallback } from 'react';
import { useApi } from '../api/context';
import { useAsync } from '../api/useAsync';
import { Async } from '../components/ui';
import { LatexText, MacrosProvider } from '../components/Markdown';
import { ReportError, ErrorReports } from '../components/ErrorReports';
import { AttemptForm } from './Conjecture';
import { EntrantName } from './Leaderboard';
import { Link } from '../routing/router';

export function SupportersPage() {
  return (
    <div className="page narrow-page">
      <header className="page-head">
        <div>
          <h1>{t('Collaborate with Mathesis')}</h1>
          <p className="lede">
            {t('We welcome collaboration on mathematical research and formalization.')}
          </p>
          <p>
            {t(
              'If you would like to explore a research project, contribute to Mathesis, or support our work, please get in touch.',
            )}
          </p>
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
                <Link to={{ kind: 'paper', record }}>{record}</Link> · {claim}
              </p>
              <h1>{s.claim.label}</h1>
              <LatexText source={s.claim.statement} />
              {reviewed ? (
                <ReportError record={record} subject={{ kind: 'statement', claim }} />
              ) : null}
              {s.claim.verified ? (
                <p className="notice">
                  <strong>{t('Lean ✓')}</strong> ·{' '}
                  <EntrantName entrant={s.claim.verified.entrant} />{' '}
                  {t(
                    reviewed
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
                    <details open>
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
                      'A target becomes available after author approval, our faithfulness check, and author confirmation.',
                    )}
                  </p>
                )}
              </section>
              {s.target ? (
                <section>
                  <h2>{t('Contribute a formalization')}</h2>
                  <Link className="button" to={{ kind: 'submit', intake: 'lean', record, claim }}>
                    {t('Submit attributed Lean work')}
                  </Link>
                  <p>
                    {t(
                      'Import Target and prove wishpool_solution : wishpool_target_prop, or wishpool_disproof : ¬ wishpool_target_prop.',
                    )}
                  </p>
                  <AttemptForm record={record} claim={claim} statement />
                </section>
              ) : null}
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
              {reviewed ? <ErrorReports record={record} /> : null}
            </article>
          </MacrosProvider>
        )}
      </Async>
    </div>
  );
}
