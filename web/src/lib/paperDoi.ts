import type { ApiClient } from '../api/client';
import { listDirectoryNotes } from './paperDirectory';
import { normaliseDoi } from './links';

export type DoiPaper = { record: string; version?: string };
const same = (left: string | null | undefined, right: string) =>
  left && normaliseDoi(left).toLowerCase() === right.toLowerCase();

/** Resolve the public identifier, then read the original version through the API.
 * Never derive work identity from a DOI suffix or a manuscript title.
 */
export async function resolvePaperDoi(
  api: ApiClient,
  doi: string,
  signal: AbortSignal,
): Promise<DoiPaper | null> {
  const papers = await listDirectoryNotes(api, {}, signal);
  const exact = papers.items.filter((paper) => same(paper.archive?.doi, doi));
  if (exact.length === 1) return { record: exact[0]!.id, version: exact[0]!.revision };
  if (exact.length > 1) throw new Error('This DOI is associated with more than one public paper.');
  if (doi.toLowerCase().startsWith('10.5281/zenodo.')) {
    const registry = await fetch('https://api.datacite.org/dois/' + encodeURIComponent(doi), {
      signal,
    });
    if (registry.ok) {
      const metadata = (await registry.json()) as {
        data?: {
          attributes?: {
            relatedIdentifiers?: { relationType: string; relatedIdentifier: string }[];
          };
        };
      };
      const concepts = (metadata.data?.attributes?.relatedIdentifiers ?? [])
        .filter((item) => item.relationType === 'IsVersionOf')
        .map((item) => item.relatedIdentifier);
      const candidates = papers.items.filter((paper) =>
        concepts.some((concept) => same(paper.archive?.concept_doi, normaliseDoi(concept))),
      );
      if (candidates.length === 1) {
        let paper = candidates[0]!;
        const visited = new Set([paper.revision]);
        while (paper.previous_version && visited.size < 100) {
          const previous = paper.previous_version;
          if (visited.has(previous)) break;
          visited.add(previous);
          const older = await api.getCollectedPaper(paper.id, previous, { signal });
          if (older.id !== paper.id || older.revision !== previous) break;
          if (same(older.archive?.doi, doi)) return { record: older.id, version: older.revision };
          paper = older;
        }
      }
    }
  }
  let before: string | null = null;
  const seen = new Set<string>();
  do {
    const page = await api.listPapers({ before, limit: 100 }, { signal });
    const matches = page.items.filter((paper) => same(paper.doi, doi));
    if (matches.length === 1) return { record: matches[0]!.record };
    if (matches.length > 1)
      throw new Error('This DOI is associated with more than one public paper.');
    before = page.next_before;
    if (before && seen.has(before))
      throw new Error('The paper directory could not finish loading.');
    if (before) seen.add(before);
  } while (before);
  // The catalog can retain an exact historical DOI omitted from the current list.
  const response = await fetch(
    'https://raw.githubusercontent.com/mathesis-papers/catalog/main/archive-index.json',
    { signal, cache: 'no-cache' },
  );
  if (!response.ok) return null;
  const index = (await response.json()) as {
    schema?: string;
    entries?: { kind: string; id: string; version: number; doi: string }[];
  };
  if (index.schema !== 'mathesis.archive-index.v1' || !Array.isArray(index.entries)) return null;
  const entries = index.entries.filter((entry) => same(entry.doi, doi));
  if (entries.length !== 1) return null;
  const entry = entries[0]!;
  if (entry.kind === 'collected') {
    const paper = await api.getCollectedPaper(entry.id, String(entry.version), { signal });
    return paper.id === entry.id &&
      paper.revision === String(entry.version) &&
      same(paper.archive?.doi, doi)
      ? { record: paper.id, version: paper.revision }
      : null;
  }
  if (entry.kind !== 'record') return null;
  const paper = await api.getPaper(entry.id, { signal });
  return same(paper.archive?.doi, doi) || same(paper.summary.doi, doi)
    ? { record: paper.summary.record }
    : null;
}
