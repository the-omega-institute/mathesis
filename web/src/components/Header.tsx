import { useState } from 'react';
import { errorMessage } from '../api/client';
import { hasRole, useSession } from '../auth/session';
import { ROLE_LABELS } from '../lib/labels';
import { Link, useRouter } from '../routing/router';
import type { Route } from '../routing/routes';
import { BrandMark } from './BrandMark';

function NavLink({ to, label, active }: { to: Route; label: string; active: boolean }) {
  return (
    <Link
      to={to}
      className={active ? 'nav-link active' : 'nav-link'}
      aria-current={active ? 'page' : undefined}
    >
      {label}
    </Link>
  );
}

export function Header() {
  const { route } = useRouter();
  const { session, person, signIn, signOut } = useSession();
  const [menuOpen, setMenuOpen] = useState(false);
  const [signOutError, setSignOutError] = useState<string | null>(null);
  const k = route.kind;
  const staff = hasRole(person, 'editor', 'reviewer', 'admin');

  return (
    <header className="site-header">
      <div className="site-header-inner">
        <div className="header-identity">
          <Link to={{ kind: 'home' }} className="wordmark">
            <BrandMark />
            Mathesis
          </Link>
          <span className="header-description">Mathematical research &amp; formal proof</span>
        </div>
        <nav className="site-nav" aria-label="Main">
          <NavLink
            to={{ kind: 'papers' }}
            label="Papers"
            active={k === 'papers' || k === 'paper'}
          />
          <NavLink
            to={{ kind: 'conjectures' }}
            label="Conjectures"
            active={k === 'conjectures' || k === 'conjecture'}
          />
          <NavLink
            to={{ kind: 'formalizations' }}
            label="Formalizations"
            active={k === 'formalizations' || k === 'statement'}
          />
          <NavLink to={{ kind: 'submit' }} label="Submit" active={k === 'submit'} />
          {person ? (
            <NavLink
              to={{ kind: 'submissions' }}
              label="My work"
              active={k === 'submissions' || k === 'submission'}
            />
          ) : null}
          {staff ? (
            <NavLink
              to={{ kind: 'queue' }}
              label="Review queue"
              active={k === 'queue' || k === 'staff_submission'}
            />
          ) : null}
          {hasRole(person, 'admin') ? (
            <NavLink to={{ kind: 'people' }} label="People" active={k === 'people'} />
          ) : null}
        </nav>
        <div className="account">
          {session.status === 'loading' ? <span className="muted small">…</span> : null}
          {session.status === 'anonymous' || session.status === 'error' ? (
            <button type="button" className="button button-quiet" onClick={() => signIn()}>
              Sign in
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
                  <p className="small">
                    {person.roles.length === 0
                      ? 'No editorial roles'
                      : person.roles.map((r) => ROLE_LABELS[r]).join(', ')}
                  </p>
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
                    Sign out
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
