import type { ReactNode } from 'react';
import { Link } from '../routing/router';
import { ExternalLink } from './ui';

/** Public request references stay in the Mathesis reader; other sources keep their own URLs. */
export function SharedRequestLink({ url, children }: { url: string; children: ReactNode }) {
  const match =
    /^https:\/\/github\.com\/the-omega-institute\/trureturing-requests\/issues\/([1-9]\d*)(?:#.*)?$/.exec(
      url,
    );
  const number = Number(match?.[1]);
  return Number.isSafeInteger(number) && number > 0 ? (
    <Link to={{ kind: 'formalizations', view: 'list', request: number }}>{children}</Link>
  ) : (
    <ExternalLink href={url}>{children}</ExternalLink>
  );
}
