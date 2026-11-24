import '../styles/formalizations.css';
import { t } from '../i18n/locale';
import { ExternalLink } from '../components/ui';
import { Link } from '../routing/router';
import { ResearchGraphPage } from './ResearchGraph';
import { useRouter } from '../routing/router';
import { useEffect } from 'react';
import { SharedLibraryGraph } from '../components/SharedLibraryGraph';
import { SharedRequests, SHARED_REQUESTS } from '../components/SharedRequests';

export function FormalizationsPage({
  msc,
  focus,
  q,
  before,
  view,
}: {
  msc?: string;
  focus?: string;
  q?: string;
  before?: string;
  view?: 'list';
}) {
  const { route, navigate } = useRouter();
  useEffect(() => {
    if (route.kind === 'research_graph')
      navigate({ kind: 'formalizations', focus, q, before }, { replace: true });
  }, [route.kind, navigate, focus, q, before]);
  return (
    <div className="page formalizations-page">
      <header className="page-head formalizations-head">
        <div>
          <h1>{t('Formalizations')}</h1>
          <p className="lede">
            {t(
              'Lean is free, open-source software that checks mathematical proofs step by step. Explore how results build on one another, find a proof to contribute, and receive credit for your work.',
            )}
          </p>
        </div>
        <nav className="formalization-actions" aria-label={t('Formalization contributions')}>
          <a className="button" href={SHARED_REQUESTS} target="_blank" rel="noopener noreferrer">
            {t('Contribute Lean work')}
          </a>
          <Link to={{ kind: 'leaderboard', category: 'formalization' }}>{t('Leaderboard')}</Link>
        </nav>
      </header>
      <nav className="section-tabs" aria-label={t('Formalization views')}>
        <Link to={{ kind: 'formalizations' }} aria-current={view !== 'list' ? 'page' : undefined}>
          {t('Dependency graph')}
        </Link>
        <Link
          to={{ kind: 'formalizations', view: 'list' }}
          aria-current={view === 'list' ? 'page' : undefined}
        >
          {t('Results and proof work')}
        </Link>
        <Link to={{ kind: 'learn_lean' }}>{t('Learn Lean')}</Link>
        <ExternalLink href="https://github.com/the-omega-institute/trureturing-requests/issues">
          {t('Shared formalization requests')}
        </ExternalLink>
      </nav>
      {view === 'list' ? (
        <SharedRequests initialQuery={q} />
      ) : (
        <>
          {!focus || focus.startsWith('module:') || focus.startsWith('library:') ? (
            <SharedLibraryGraph
              msc={msc}
              focus={focus?.startsWith('module:') ? focus.slice(7) : undefined}
            />
          ) : (
            <ResearchGraphPage embedded focus={focus} q={q} before={before} />
          )}
        </>
      )}
    </div>
  );
}
