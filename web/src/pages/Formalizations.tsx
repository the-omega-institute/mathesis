import '../styles/formalizations.css';
import { t } from '../i18n/locale';
import { ResearchGraphPage } from './ResearchGraph';
import { useRouter } from '../routing/router';
import { useEffect } from 'react';
import { SharedLibraryGraph } from '../components/SharedLibraryGraph';
import { SharedRequests } from '../components/SharedRequests';
import { FormalizationNav } from '../components/FormalizationNav';

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
      </header>
      <FormalizationNav active={view === 'list' ? 'requests' : 'graph'} />
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
