import { t } from '../i18n/locale';
import { publicationSubjects } from '../lib/projectNoteSubjects';
import type { ResearchEntry } from '../lib/researchEntries';
import { findSubject } from '../lib/subjects';
import { Link } from '../routing/router';
import { InlineLatex } from './Markdown';
import { ExternalLink } from './ui';

/** The directory is for discovery; authors and full citations remain on the paper. */
export function PaperDirectoryEntry({ entry }: { entry: ResearchEntry }) {
  const paper = entry.kind === 'paper' ? entry.paper : undefined;
  const note = entry.kind === 'note' ? entry.note : undefined;
  const record = paper?.record ?? note!.id;
  const title = paper?.title ?? note!.title;
  const doi = paper?.doi ?? note?.archive?.doi;
  const pdf = note?.pdf.url;
  const codes = publicationSubjects(record, paper?.msc ?? note?.msc);
  const subjects = [...new Set(codes.map((code) => code.slice(0, 2)))].map((code) => ({
    code,
    title: findSubject(code)?.title ?? `MSC ${code}`,
  }));
  return (
    <article className="paper-entry paper-directory-entry">
      <div className="paper-directory-body">
        {subjects.length ? (
          <ul className="paper-directory-subjects" aria-label={t('Subjects')}>
            {subjects.map((subject) => (
              <li key={subject.code}>
                <Link to={{ kind: 'subject', msc: subject.code }}>{t(subject.title)}</Link>
              </li>
            ))}
          </ul>
        ) : null}
        <h2>
          <Link to={{ kind: 'paper', record }}>
            <InlineLatex source={title} />
          </Link>
        </h2>
        {doi ? (
          <p className="paper-directory-doi">
            DOI <ExternalLink href={`https://doi.org/${doi}`}>{doi}</ExternalLink>
          </p>
        ) : null}
      </div>
      {pdf ? (
        <a className="paper-directory-pdf" href={pdf}>
          {t('Read PDF')}
        </a>
      ) : (
        <Link className="paper-directory-pdf" to={{ kind: 'paper', record }}>
          {t('View paper')}
        </Link>
      )}
    </article>
  );
}
