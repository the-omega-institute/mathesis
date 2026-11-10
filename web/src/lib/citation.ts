import type { PaperSummary } from '../api/types';
import { formatDate } from './format';
import { formatAuthors } from './labels';
import { sourceHref } from './links';

/** A plain-text citation for an accepted paper's public record. */
export function paperCitation(paper: PaperSummary, origin: string): string {
  const authors = formatAuthors(paper.authors);
  const title = paper.title.trim().replace(/\.$/, '');
  const publicationDoiHref = paper.publication_doi
    ? sourceHref({ kind: 'doi', locator: paper.publication_doi })
    : null;
  const authorDoiHref = paper.doi ? sourceHref({ kind: 'doi', locator: paper.doi }) : null;
  const dois = [
    paper.publication_doi
      ? `doi:${paper.publication_doi}${publicationDoiHref ? ` (${publicationDoiHref})` : ''}.`
      : null,
    paper.doi ? `author DOI: ${paper.doi}${authorDoiHref ? ` (${authorDoiHref})` : ''}.` : null,
  ];
  return [
    `${authors}. ${title}.`,
    `Mathesis record ${paper.record}${paper.accepted_at ? `, accepted ${formatDate(paper.accepted_at)}` : ''}.`,
    ...dois,
    `${origin}/papers/${encodeURIComponent(paper.record)}`,
  ]
    .filter((part) => part !== null)
    .join(' ');
}
