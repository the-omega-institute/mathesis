import type { Author } from '../api/types';
import { orcidHref, safeHttpUrl } from '../lib/links';
import { texToText } from '../lib/texText';
import { ExternalLink } from './ui';
import { Link } from '../routing/router';

export function AuthorLine({ authors }: { authors: readonly Author[] }) {
  return (
    <ul className="author-line">
      {authors.map((a, i) => {
        const orcid = a.orcid ? orcidHref(a.orcid) : null;
        const scholar = a.google_scholar ? safeHttpUrl(a.google_scholar) : null;
        const website = a.website ? safeHttpUrl(a.website) : null;
        return (
          <li key={`${a.name}-${i}`}>
            <Link className="author-name" to={{ kind: 'author', name: a.name }}>
              {texToText(a.name)}
            </Link>
            {orcid ? (
              <>
                {' '}
                <ExternalLink href={orcid}>
                  <span className="orcid" aria-label={`ORCID iD of ${texToText(a.name)}`}>
                    iD
                  </span>
                </ExternalLink>
              </>
            ) : null}
            {a.affiliation ? <span className="affiliation">{a.affiliation}</span> : null}
            {scholar ? (
              <>
                {' · '}
                <ExternalLink href={scholar}>Google Scholar</ExternalLink>
              </>
            ) : null}
            {website ? (
              <>
                {' · '}
                <ExternalLink href={website}>website</ExternalLink>
              </>
            ) : null}
          </li>
        );
      })}
    </ul>
  );
}
