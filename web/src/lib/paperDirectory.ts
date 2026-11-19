import type { ApiClient, PaperQuery } from '../api/client';
import type { CollectedPaper, Listing } from '../api/types';

/** The collection has cursor pagination. Follow it before applying editorial subjects,
 * since older deposits may have no supplied MSC metadata. All other filters stay
 * on the server and the abort signal follows every page request.
 */
export async function listDirectoryNotes(
  api: ApiClient,
  query: PaperQuery,
  signal: AbortSignal,
): Promise<Listing<CollectedPaper>> {
  const filters = { ...query };
  delete filters.msc;
  delete filters.before;
  delete filters.limit;
  const items: CollectedPaper[] = [];
  const cursors = new Set<string>();
  let before: string | null = null;
  do {
    signal.throwIfAborted();
    const page = await api.listCollectedPapers({ ...filters, before, limit: 100 }, { signal });
    items.push(...page.items);
    before = page.next_before;
    if (before !== null) {
      if (cursors.has(before)) throw new Error('The paper directory could not finish loading.');
      cursors.add(before);
    }
  } while (before !== null);
  // Merge live revisions over historical notes before filtering subjects in the
  // reader, so a superseded classification cannot reappear from the snapshot.
  return { items, next_before: null };
}
