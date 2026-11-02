import { t } from '../i18n/locale';
import { useId, useState } from 'react';
import { useRouter } from '../routing/router';

export function ArchiveSearch() {
  const [query, setQuery] = useState('');
  const { navigate } = useRouter();
  const id = useId();
  return (
    <form
      className="global-search"
      role="search"
      aria-label={t('Search the archive')}
      onSubmit={(event) => {
        event.preventDefault();
        navigate({ kind: 'papers', ...(query.trim() ? { q: query.trim() } : {}) });
      }}
    >
      <label htmlFor={id} className="visually-hidden">
        {t('Search all research')}
      </label>
      <input
        id={id}
        type="search"
        maxLength={300}
        value={query}
        onChange={(event) => setQuery(event.target.value)}
        placeholder={t('Title, author, keyword…')}
      />
      <button type="submit" className="global-search-submit" aria-label={t('Search')}>
        <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
          <circle cx="10.5" cy="10.5" r="6.5" />
          <path d="m16 16 4.5 4.5" />
        </svg>
      </button>
    </form>
  );
}
