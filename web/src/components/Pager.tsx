import { t } from '../i18n/locale';
import type { Paged } from '../api/usePaged';

export function Pager<T>({
  paged,
  labels = { newer: '← Newer', older: 'Older →' },
}: {
  paged: Paged<T>;
  /** Button text, e.g. `← Previous` / `Next →` for listings not ordered by date. */
  labels?: { newer: string; older: string };
}) {
  if (!paged.hasNewer && !paged.hasOlder) return null;
  return (
    <nav className="pager" aria-label={t('Pagination')}>
      <button
        type="button"
        className="button button-quiet"
        disabled={!paged.hasNewer}
        onClick={paged.newer}
      >
        {t(labels.newer)}
      </button>
      <span className="muted small">
        {t('Page ')}
        {paged.page}
      </span>
      <button
        type="button"
        className="button button-quiet"
        disabled={!paged.hasOlder}
        onClick={paged.older}
      >
        {t(labels.older)}
      </button>
    </nav>
  );
}
