import { useCallback, type ReactNode } from 'react';
import { useApi } from '../api/context';
import { useAsync } from '../api/useAsync';
import type { PaperMaterials, ResearchResult } from '../api/types';
import { t } from '../i18n/locale';
import { resultLabel } from '../lib/statements';
import { Link } from '../routing/router';
import { InlineLatex, LatexText, MacrosProvider } from './Markdown';
import { ExternalLink } from './ui';
import { SharedRequestLink } from './SharedRequestLink';
import './paper-research.css';

/** Public source statements and shared work. Private review advice never enters this view. */
export function PaperResearch({
  work,
  version,
  origin,
  materials,
  fallback,
  leanSource,
  macros,
}: {
  work: string;
  version: number;
  origin: string;
  materials?: PaperMaterials;
  fallback?: ReactNode;
  leanSource?: string;
  macros?: Record<string, string>;
}) {
  const api = useApi();
  const data = useAsync(
    useCallback(
      (signal: AbortSignal) => api.paperResearch(work, version, { signal }),
      [api, work, version],
    ),
  );
  // A stale or wrong-work response must never get attached to this reading.
  const reading = data.value?.work === work && data.value.version === version ? data.value : null;
  const results = reading?.results ?? [];
  const main = results.filter((row) => row.role !== 'supporting');
  const supporting = results.filter((row) => row.role === 'supporting');
  const rows = (items: ResearchResult[]) => (
    <ul className="paper-research-list">
      {items.map((row) => (
        <li key={row.claim}>
          <h3>
            <Link to={{ kind: 'result', record: work, claim: row.claim, version: String(version) }}>
              <InlineLatex
                source={resultLabel({ id: row.claim, label: row.name, kind: row.kind })}
              />
            </Link>
          </h3>
          {row.state !== 'published' ? (
            <span className="small">
              {t(row.state === 'verified' ? 'Verified in Lean' : 'Disproved in Lean')}
            </span>
          ) : null}
          <details className="result-statement">
            <summary>{t('Read the statement')}</summary>
            <LatexText source={row.statement} />
          </details>
        </li>
      ))}
    </ul>
  );
  const requests = reading?.requests ?? [];
  const missingLinks = (materials?.formalization_requests ?? []).filter(
    (link) => !requests.some((row) => row.request.url === link.url),
  );
  const source = reading?.lean_source ?? leanSource;
  return (
    <MacrosProvider macros={reading?.macros ?? macros}>
      {results.length ? (
        <section className="paper-research" aria-labelledby="results-h">
          <h2 id="results-h">{t('Results')}</h2>
          {rows(main.slice(0, 3))}
          {main.length > 3 || supporting.length ? (
            <details className="paper-research-more">
              <summary>{t('More results and supporting statements')}</summary>
              {rows([...main.slice(3), ...supporting])}
            </details>
          ) : null}
        </section>
      ) : reading ? null : (
        fallback
      )}
      <section className="paper-research" aria-labelledby="further-work-h">
        <h2 id="further-work-h">{t('Continue this research')}</h2>
        {reading?.questions.length ? (
          <>
            <h3>{t('Questions in this paper')}</h3>
            {rows(reading.questions)}
          </>
        ) : null}
        {requests.length || missingLinks.length ? (
          <>
            <h3>
              {t(
                requests.some((row) => row.association === 'repository')
                  ? 'Related formalization work'
                  : 'Formalization and contributions',
              )}
            </h3>
            <ul className="paper-request-list">
              {requests.map(({ request }) => (
                <li key={request.number}>
                  <SharedRequestLink url={request.url}>
                    <InlineLatex source={request.title} />
                  </SharedRequestLink>
                  <p className="entry-meta">
                    {t(
                      request.state === 'open'
                        ? 'Open for contribution'
                        : request.state === 'claimed'
                          ? 'Claimed'
                          : request.state === 'merged'
                            ? 'Merged contribution'
                            : 'Closed request',
                    )}
                    {request.claimants.map((person) => (
                      <span key={person.login}>
                        {' '}
                        · <ExternalLink href={person.url}>{person.login}</ExternalLink>
                      </span>
                    ))}
                  </p>
                  {request.completions.map((done) => (
                    <p key={done.url} className="small">
                      <ExternalLink href={done.url}>{t('Read the contribution')}</ExternalLink> ·{' '}
                      <ExternalLink href={done.contributor.url}>
                        {done.contributor.login}
                      </ExternalLink>
                    </p>
                  ))}
                </li>
              ))}
              {missingLinks.map((request) => (
                <li key={request.url}>
                  <SharedRequestLink url={request.url}>{request.label}</SharedRequestLink>
                </li>
              ))}
            </ul>
            <p className="small">
              {t(
                'Claim a task in its shared discussion and link your proof pull request there. The request records its scope, contributors and outcome.',
              )}
            </p>
          </>
        ) : null}
        {source ? (
          <p>
            <ExternalLink href={source}>{t('Read the Lean source')}</ExternalLink>
          </p>
        ) : null}
        <p className="paper-research-actions">
          <Link to={{ kind: 'submit', intake: 'conjecture', origin }}>
            {t('Propose a further question')}
          </Link>
          <Link to={{ kind: 'submit', intake: 'lean' }}>{t('Find formalization work')}</Link>
        </p>
      </section>
    </MacrosProvider>
  );
}
