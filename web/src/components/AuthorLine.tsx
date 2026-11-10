import { t } from '../i18n/locale';
import type { Author } from '../api/types';
import { canonicalAuthorName } from '../lib/authorName';
import { orcidHref, safeHttpUrl } from '../lib/links';
import { texToText } from '../lib/texText';
import { ExternalLink } from './ui';
import { Link } from '../routing/router';

export function AuthorLine({
  authors,
  compact = false,
}: {
  authors: readonly Author[];
  compact?: boolean;
}) {
  return (
    <ul className={compact ? 'author-line is-compact' : 'author-line'}>
      {authors.map((a, i) => {
        // One written form for the text and for the archive query it links to.
        const name = canonicalAuthorName(a.name);
        const orcid = a.orcid ? orcidHref(a.orcid) : null;
        const scholar = a.google_scholar ? safeHttpUrl(a.google_scholar) : null;
        const website = a.website ? safeHttpUrl(a.website) : null;
        return (
          <li key={`${name}-${i}`}>
            <Link className="author-name" to={{ kind: 'papers', author: name }}>
              {texToText(name)}
            </Link>
            {orcid && !compact ? (
              <>
                {' '}
                <ExternalLink href={orcid}>
                  <span className="orcid" aria-label={`ORCID iD of ${texToText(name)}`}>
                    {t('iD')}
                  </span>
                </ExternalLink>
              </>
            ) : null}
            {a.affiliation && !compact ? (
              <span className="affiliation">{a.affiliation}</span>
            ) : null}
            {scholar && !compact ? (
              <>
                {' · '}
                <ExternalLink href={scholar}>{t('Google Scholar')}</ExternalLink>
              </>
            ) : null}
            {website && !compact ? (
              <>
                {' · '}
                <ExternalLink href={website}>{t('website')}</ExternalLink>
              </>
            ) : null}
          </li>
        );
      })}
    </ul>
  );
}
