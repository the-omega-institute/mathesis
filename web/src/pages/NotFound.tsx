import { t } from '../i18n/locale';
import { Link } from '../routing/router';

export function NotFoundPage({ path }: { path: string }) {
  return (
    <div className="page narrow-page">
      <h1>{t('Page not found')}</h1>
      <p>
        {t('Nothing lives at ')}
        <code>{path}</code>.
      </p>
      <p>
        <Link to={{ kind: 'home' }}>{t('Return to the front page')}</Link>
      </p>
    </div>
  );
}
