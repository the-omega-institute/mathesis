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
      aria-label="Search the archive"
      onSubmit={(event) => {
        event.preventDefault();
        navigate({ kind: 'papers', ...(query.trim() ? { q: query.trim() } : {}) });
      }}
    >
      <label htmlFor={id} className="visually-hidden">
        Search all research
      </label>
      <input
        id={id}
        type="search"
        maxLength={300}
        value={query}
        onChange={(event) => setQuery(event.target.value)}
        placeholder="Title, author, keyword…"
      />
      <button type="submit" className="button button-quiet">
        Search
      </button>
    </form>
  );
}
