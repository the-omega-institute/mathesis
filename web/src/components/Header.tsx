import { t, setLocale, useLocale } from '../i18n/locale';
import { ArchiveSearch } from './ArchiveSearch';
import { useState } from 'react';
import { errorMessage } from '../api/client';
import { hasRole, useSession } from '../auth/session';
import { Link, useRouter } from '../routing/router';
import type { Route } from '../routing/routes';
import { BrandMark } from './BrandMark';
import { GitHubLink } from './GitHubLink';

function NavLink({ to, label, active }: { to: Route; label: string; active: boolean }) {
  return (
    <Link
      to={to}
      className={active ? 'nav-link active' : 'nav-link'}
      aria-current={active ? 'page' : undefined}
    >
      {t(label)}
    </Link>
  );
}

export function Header() {
  const locale = useLocale();
  const { route } = useRouter();
  const { session, person, signIn, signOut } = useSession();
  const [menuOpen, setMenuOpen] = useState(false);
  const [signOutError, setSignOutError] = useState<string | null>(null);
  const k = route.kind;
  const staff = hasRole(person, 'editor', 'reviewer', 'admin');

  return (
    <header className="site-header">
      <div className={`site-header-inner${staff ? ' has-staff-navigation' : ''}`}>
        <Link to={{ kind: 'home' }} className="wordmark">
          <BrandMark />
          Mathesis
        </Link>
        <nav className="site-nav" aria-label={t('Main')}>
          <NavLink
            to={{ kind: 'papers' }}
            label={t('Papers')}
            active={k === 'papers' || k === 'paper' || k === 'result'}
          />
          <NavLink
            to={{ kind: 'conjectures' }}
            label={t('Conjectures')}
            active={k === 'conjectures' || k === 'conjecture'}
          />
          <NavLink
            to={{ kind: 'formalizations' }}
            label={t('Formalizations')}
            active={
              k === 'formalizations' ||
              k === 'research_graph' ||
              k === 'statement' ||
              k === 'learn_lean'
            }
          />
          <NavLink to={{ kind: 'submit' }} label={t('Submit')} active={k === 'submit'} />
          {person ? (
            <NavLink
              to={{ kind: 'submissions' }}
              label={t('My work')}
              active={k === 'submissions' || (k === 'submission' && !route.staff)}
            />
          ) : null}
          {staff ? (
            <NavLink
              to={{ kind: 'queue' }}
              label={t('Review queue')}
              active={k === 'queue' || (k === 'submission' && !!route.staff)}
            />
          ) : null}
          {hasRole(person, 'admin') ? (
            <NavLink to={{ kind: 'people' }} label={t('Accounts')} active={k === 'people'} />
          ) : null}
        </nav>
        <ArchiveSearch />
        <GitHubLink compact />
        <div className="language-choice" role="group" aria-label={t('Language')}>
          <button
            type="button"
            lang="en"
            aria-pressed={locale === 'en'}
            onClick={() => setLocale('en')}
          >
            English
          </button>
          <span aria-hidden="true">/</span>
          <button
            type="button"
            lang="zh-Hans"
            aria-pressed={locale === 'zh'}
            onClick={() => setLocale('zh')}
          >
            中文
          </button>
        </div>
        <div className="account">
          {session.status === 'loading' ? <span className="muted small">…</span> : null}
          {session.status === 'anonymous' || session.status === 'error' ? (
            <button type="button" className="button button-quiet" onClick={() => signIn()}>
              {t('Sign in')}
            </button>
          ) : null}
          {person ? (
            <div className="user-menu">
              <button
                type="button"
                className="user-button"
                aria-expanded={menuOpen}
                aria-haspopup="true"
                onClick={() => setMenuOpen((open) => !open)}
              >
                {person.display_name}
              </button>
              {menuOpen ? (
                <div className="user-panel">
                  <p className="user-name">{person.display_name}</p>
                  {person.email ? <p className="muted small">{person.email}</p> : null}
                  {hasRole(person, 'admin') ? (
                    <p className="small muted">{t('Administrator')}</p>
                  ) : null}
                  <button
                    type="button"
                    className="button button-quiet"
                    onClick={() => {
                      setSignOutError(null);
                      signOut().then(
                        () => setMenuOpen(false),
                        (e: unknown) => setSignOutError(errorMessage(e)),
                      );
                    }}
                  >
                    {t('Sign out')}
                  </button>
                  {signOutError ? <p className="form-error">{signOutError}</p> : null}
                </div>
              ) : null}
            </div>
          ) : null}
        </div>
      </div>
    </header>
  );
}
