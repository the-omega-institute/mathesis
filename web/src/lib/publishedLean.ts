import type { CollectedPaper } from '../api/types';
import { safeHttpUrl } from './links';

/** Published source is evidence in its own right, without claiming theorem-level coverage. */
export function publishedLeanSource(paper: Pick<CollectedPaper, 'lean'>): string | undefined {
  const source = paper.lean;
  if (!source || !/^[a-f\d]{40}$/i.test(source.revision)) return;
  const href = safeHttpUrl(source.url);
  if (!href) return;
  const url = new URL(href);
  if (
    url.origin !== 'https://github.com' ||
    !url.pathname.includes(`/blob/${source.revision}/`) ||
    !url.pathname.endsWith('.lean')
  )
    return;
  return href;
}
