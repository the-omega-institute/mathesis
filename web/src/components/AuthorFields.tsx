import { t } from '../i18n/locale';
import type { Author } from '../api/types';
import { Field } from './ui';
import { orcidHref, safeHttpUrl } from '../lib/links';

export type AuthorDraft = {
  name: string;
  affiliation: string;
  orcid: string;
  googleScholar: string;
  website: string;
};

export function emptyAuthor(): AuthorDraft {
  return { name: '', affiliation: '', orcid: '', googleScholar: '', website: '' };
}

export function authorHasContent(author: AuthorDraft): boolean {
  return Object.values(author).some((value) => value.trim() !== '');
}

export function suppliedAuthors(
  drafts: AuthorDraft[],
): { ok: true; value: Author[] } | { ok: false; error: string } {
  const value: Author[] = [];
  for (const [index, draft] of drafts.entries()) {
    const number = index + 1;
    const name = draft.name.trim();
    if (!name) return { ok: false, error: t('Give a name for author {number}.', { number }) };
    if (draft.orcid.trim() && !orcidHref(draft.orcid.trim())) {
      return { ok: false, error: t('The ORCID for author {number} is not valid.', { number }) };
    }
    for (const [label, raw] of [
      ['Google Scholar URL', draft.googleScholar],
      ['Website or other link', draft.website],
    ] as const) {
      if (raw.trim() && !safeHttpUrl(raw.trim())) {
        return {
          ok: false,
          error: t('{label} for author {number} must be an http(s) URL.', {
            label: t(label),
            number,
          }),
        };
      }
    }
    value.push({
      name,
      ...(draft.affiliation.trim() ? { affiliation: draft.affiliation.trim() } : {}),
      ...(draft.orcid.trim() ? { orcid: draft.orcid.trim() } : {}),
      ...(draft.googleScholar.trim() ? { google_scholar: draft.googleScholar.trim() } : {}),
      ...(draft.website.trim() ? { website: draft.website.trim() } : {}),
    });
  }
  return { ok: true, value };
}

export function AuthorFields({
  index,
  author,
  canRemove,
  onChange,
  onRemove,
}: {
  index: number;
  author: AuthorDraft;
  canRemove: boolean;
  onChange: (author: AuthorDraft) => void;
  onRemove: () => void;
}) {
  const patch = (key: keyof AuthorDraft, value: string) => onChange({ ...author, [key]: value });
  return (
    <fieldset className="author-fields">
      <legend>
        {t('Author ')}
        {index + 1}
      </legend>
      <div className="form-row">
        <Field label={t('Name')} htmlFor={`author-${index}-name`}>
          <input
            id={`author-${index}-name`}
            value={author.name}
            onChange={(e) => patch('name', e.target.value)}
            autoComplete="name"
          />
        </Field>
        <Field label={t('Affiliation')} htmlFor={`author-${index}-affiliation`}>
          <input
            id={`author-${index}-affiliation`}
            value={author.affiliation}
            onChange={(e) => patch('affiliation', e.target.value)}
            placeholder={t('University or institute')}
            autoComplete="organization"
          />
        </Field>
      </div>
      <div className="author-profiles">
        <p className="hint">{t('Academic profiles (optional)')}</p>
        <div className="form-row">
          <Field label="ORCID" htmlFor={`author-${index}-orcid`}>
            <input
              id={`author-${index}-orcid`}
              value={author.orcid}
              onChange={(e) => patch('orcid', e.target.value)}
              placeholder="0000-0002-1825-0097"
              inputMode="text"
            />
          </Field>
          <Field label={t('Google Scholar')} htmlFor={`author-${index}-scholar`}>
            <input
              id={`author-${index}-scholar`}
              type="url"
              value={author.googleScholar}
              onChange={(e) => patch('googleScholar', e.target.value)}
              placeholder="https://scholar.google.com/citations?..."
            />
          </Field>
        </div>
        <Field label={t('Website or other link')} htmlFor={`author-${index}-website`}>
          <input
            id={`author-${index}-website`}
            type="url"
            value={author.website}
            onChange={(e) => patch('website', e.target.value)}
            placeholder="https://example.edu/~name"
          />
        </Field>
      </div>
      {canRemove ? (
        <button type="button" className="button button-quiet button-small" onClick={onRemove}>
          {t('Remove author')}
        </button>
      ) : null}
    </fieldset>
  );
}
