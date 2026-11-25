import { t } from '../i18n/locale';
import { Link } from '../routing/router';

export function FormalizationNav({
  active,
}: {
  active: 'graph' | 'requests' | 'leaderboard' | 'learn';
}) {
  return (
    <nav className="section-tabs" aria-label={t('Formalization views')}>
      <Link to={{ kind: 'formalizations' }} aria-current={active === 'graph' ? 'page' : undefined}>
        {t('Existing DAG')}
      </Link>
      <Link
        to={{ kind: 'formalizations', view: 'list' }}
        aria-current={active === 'requests' ? 'page' : undefined}
      >
        {t('Awaiting formalization')}
      </Link>
      <Link
        to={{ kind: 'leaderboard', category: 'formalization' }}
        aria-current={active === 'leaderboard' ? 'page' : undefined}
      >
        {t('Leaderboard')}
      </Link>
      <Link to={{ kind: 'learn_lean' }} aria-current={active === 'learn' ? 'page' : undefined}>
        {t('Learn Lean')}
      </Link>
    </nav>
  );
}
