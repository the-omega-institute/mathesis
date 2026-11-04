import { t } from '../i18n/locale';
import { findSubject, subjectGroups } from '../lib/subjects';
import { Link } from '../routing/router';
import { NotFoundPage } from './NotFound';
import { PapersPage } from './Papers';
import '../styles/subjects.css';

export function SubjectsPage() {
  return (
    <div className="page subjects-page">
      <Link to={{ kind: 'papers' }}>{t('Research archive')}</Link>
      <header className="page-head">
        <div>
          <h1>{t('Browse by subject')}</h1>
          <p className="lede">{t('Browse papers by mathematical subject.')}</p>
          <p className="muted">
            {t('Subject headings follow the Mathematics Subject Classification (MSC 2020).')}
          </p>
        </div>
      </header>
      <div className="subject-directory">
        {subjectGroups.map((group) => (
          <section key={group.title} className="subject-group">
            <h2>{t(group.title)}</h2>
            <ul>
              {group.subjects.map((subject) => (
                <li key={subject.code}>
                  <Link to={{ kind: 'subject', msc: subject.code }}>{t(subject.title)}</Link>
                </li>
              ))}
            </ul>
          </section>
        ))}
      </div>
    </div>
  );
}

export function SubjectPage({ msc }: { msc: string }) {
  const subject = findSubject(msc);
  if (!subject) return <NotFoundPage path={`/subjects/${msc}`} />;
  return <PapersPage key={msc} initialFilters={{ msc }} subject={subject} />;
}
