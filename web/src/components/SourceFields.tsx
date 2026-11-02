import { t } from '../i18n/locale';
import type { Source, SourceKind } from '../api/types';
import { SOURCE_KIND_LABELS } from '../lib/labels';
import { sourceHref } from '../lib/links';
import { ExternalLink } from './ui';

const PLACEHOLDERS: { [K in SourceKind]: string } = {
  arxiv: '2401.01234',
  hexagon: 'https://…',
  zenodo: '10.5281/zenodo.1234567',
  doi: '10.1000/xyz123',
  oeis: 'A000045',
  url: 'https://…',
  named_work: 'Work named in the paper or report',
  personal: 'Who, when',
};

/** Kind + locator (+ optional year) with a live preview of where the link resolves. */
export function SourceFields({
  idPrefix,
  value,
  kinds,
  onChange,
  withYear = false,
  required = true,
}: {
  idPrefix: string;
  value: Source;
  kinds: readonly SourceKind[];
  onChange: (source: Source) => void;
  withYear?: boolean;
  required?: boolean;
}) {
  const href = value.locator.trim() ? sourceHref(value) : null;
  return (
    <div className="source-fields">
      <div className="field">
        <label htmlFor={`${idPrefix}-kind`}>{t('Kind')}</label>
        <select
          id={`${idPrefix}-kind`}
          value={value.kind}
          onChange={(e) => onChange({ ...value, kind: e.target.value as SourceKind })}
        >
          {kinds.map((k) => (
            <option key={k} value={k}>
              {t(SOURCE_KIND_LABELS[k])}
            </option>
          ))}
        </select>
      </div>
      <div className="field grow">
        <label htmlFor={`${idPrefix}-locator`}>{t('Locator')}</label>
        <input
          id={`${idPrefix}-locator`}
          value={value.locator}
          required={required}
          placeholder={t(PLACEHOLDERS[value.kind])}
          onChange={(e) => onChange({ ...value, locator: e.target.value })}
        />
      </div>
      {withYear ? (
        <div className="field narrow">
          <label htmlFor={`${idPrefix}-year`}>{t('Year')}</label>
          <input
            id={`${idPrefix}-year`}
            inputMode="numeric"
            value={value.year ?? ''}
            onChange={(e) => {
              const digits = e.target.value.replace(/\D/g, '').slice(0, 4);
              const next: Source = { kind: value.kind, locator: value.locator };
              if (digits !== '') next.year = Number(digits);
              onChange(next);
            }}
          />
        </div>
      ) : null}
      <p className="hint source-preview">
        {value.locator.trim() === '' ? (
          ' '
        ) : href ? (
          <>
            {t('Resolves to ')}
            <ExternalLink href={href}>{href}</ExternalLink>
          </>
        ) : (
          t('Shown as text; this locator does not resolve to a public link.')
        )}
      </p>
    </div>
  );
}
