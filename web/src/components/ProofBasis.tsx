import { t } from '../i18n/locale';
import type { ProofBasis } from '../api/types';
import { safeHttpUrl } from '../lib/links';
import { ExternalLink } from './ui';

/**
 * Whether a verified proof references declarations of a prepared library or is new Lean.
 * The receipt records direct references at a pinned revision; nothing here compares the
 * meaning of a library declaration with the paper.
 */
export function ProofBasisNote({
  basis,
  verdict = 'proved',
  compact = false,
}: {
  basis?: ProofBasis | null;
  verdict?: 'proved' | 'disproved';
  /** A list entry: the label and the pinned library; declarations stay on the result page. */
  compact?: boolean;
}) {
  if (!basis) return null;
  const proved = verdict === 'proved';
  if (basis.kind === 'new_lean')
    return (
      <p className="small proof-basis">
        <strong>{t(proved ? 'New Lean proof' : 'New Lean disproof')}</strong>
        {compact ? null : (
          <>
            {' · '}
            <span>{t('The checked proof references no declaration of a prepared library.')}</span>
          </>
        )}
      </p>
    );
  const label = t(
    proved ? 'Proved from library declarations' : 'Disproved from library declarations',
  );
  if (compact)
    return (
      <p className="small proof-basis">
        <strong>{label}</strong>
        {basis.libraries.map((library) => (
          <span key={library.name}>
            {' · '}
            <span>
              {t('{library} at revision {revision}', {
                library: library.name,
                revision: library.revision,
              })}
            </span>
          </span>
        ))}
      </p>
    );
  return (
    <details className="proof-basis" open={basis.declarations.length <= 4}>
      <summary>
        <strong>{label}</strong>
      </summary>
      <div>
        <p className="small">
          {t(
            proved
              ? 'The Lean target follows from these library declarations together with the Lean submitted with the proof.'
              : 'The negation of the Lean target follows from these library declarations together with the Lean submitted with the proof.',
          )}{' '}
          {t(
            'The independent verifier lists the declarations that the proof’s own statements and proof terms reference directly, at the pinned library revision. This records references; it does not compare what those declarations state with the paper.',
          )}
        </p>
        <ul className="small">
          {basis.libraries.map((library) => (
            <li key={library.name}>
              {t('{library} at revision {revision}', {
                library: library.name,
                revision: library.revision,
              })}
            </li>
          ))}
        </ul>
        <ul className="small">
          {basis.declarations.map((declaration) => {
            const href = declaration.source ? safeHttpUrl(declaration.source) : null;
            return (
              <li key={`${declaration.module}:${declaration.name}`}>
                <code>{declaration.name}</code>
                {' · '}
                {href ? (
                  <ExternalLink href={href}>
                    <span aria-label={t('Library source of {name}', { name: declaration.name })}>
                      {declaration.module}
                    </span>
                  </ExternalLink>
                ) : (
                  <span>{declaration.module}</span>
                )}
              </li>
            );
          })}
        </ul>
        {basis.total > basis.declarations.length ? (
          <p className="small">
            {t('Showing {shown} of {total} referenced library declarations.', {
              shown: basis.declarations.length,
              total: basis.total,
            })}
          </p>
        ) : null}
        {basis.complete ? null : (
          <p className="small">
            {t(
              'The inspection stopped at its bound, so further library declarations may be referenced.',
            )}
          </p>
        )}
      </div>
    </details>
  );
}
