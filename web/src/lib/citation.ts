import type { PaperSummary, PublicClaim } from '../api/types';
import { formatDate } from './format';
import { formatAuthors } from './labels';
import { sourceHref } from './links';
import { resultLabel } from './statements';
import { routePath } from '../routing/routes';

export type CitationLocator = { claim?: PublicClaim; version?: number };

export function citationUrl(
  paper: PaperSummary,
  origin: string,
  locator: CitationLocator = {},
): string {
  return (
    origin.replace(/\/$/, '') +
    routePath(
      locator.claim
        ? { kind: 'result', record: paper.record, claim: locator.claim.id }
        : { kind: 'paper', record: paper.record },
    )
  );
}

/** A plain-text citation for an accepted paper's public record. */
export function paperCitation(
  paper: PaperSummary,
  origin: string,
  locator: CitationLocator = {},
): string {
  const authors = formatAuthors(paper.authors);
  const title = paper.title.trim().replace(/\.$/, '');
  const doiHref = paper.doi ? sourceHref({ kind: 'doi', locator: paper.doi }) : null;
  return [
    `${authors}. ${title}.`,
    `Mathesis record ${paper.record}${paper.accepted_at ? `, accepted ${formatDate(paper.accepted_at)}` : ''}.`,
    locator.claim
      ? `${resultLabel(locator.claim)}${locator.version ? `, version ${locator.version}` : ''}.`
      : locator.version
        ? `Version ${locator.version}.`
        : null,
    paper.doi ? `doi:${paper.doi}${doiHref ? ` (${doiHref})` : ''}.` : null,
    citationUrl(paper, origin, locator),
  ]
    .filter((part) => part !== null)
    .join(' ');
}

/** Keep balanced LaTeX, while containing malformed metadata inside one BibTeX field. */
function bibtexValue(value: string): string {
  const result: string[] = [];
  const openings: number[] = [];
  const text = value
    .replace(/[\r\n\t]+/g, ' ')
    .split('')
    .filter((character) => character.charCodeAt(0) >= 32 && character.charCodeAt(0) !== 127)
    .join('');
  for (let i = 0; i < text.length; i++) {
    const character = text[i]!;
    if (character === '\\') {
      result.push(i + 1 < text.length ? `\\${text[++i]}` : '\\textbackslash{}');
    } else if (character === '{') {
      openings.push(result.length);
      result.push(character);
    } else if (character === '}') {
      result.push(openings.length ? (openings.pop(), '}') : '\\}');
    } else {
      result.push('%&#'.includes(character) ? `\\${character}` : character);
    }
  }
  for (const index of openings) result[index] = '\\{';
  return result.join('');
}

export function paperBibtex(
  paper: PaperSummary,
  origin: string,
  locator: CitationLocator = {},
): string {
  const key = `${paper.record}${locator.claim ? `-${locator.claim.id}` : ''}`.replace(
    /[^a-zA-Z0-9_-]/g,
    '-',
  );
  const year = paper.accepted_at?.match(/^\d{4}/)?.[0];
  const arxiv = /^10\.48550\/arxiv\.(.+)$/i.exec(paper.doi ?? '')?.[1];
  const note = [
    `Mathesis record ${paper.record}`,
    paper.accepted_at ? `accepted ${formatDate(paper.accepted_at)}` : null,
    locator.version ? `version ${locator.version}` : null,
    locator.claim ? resultLabel(locator.claim) : null,
  ]
    .filter(Boolean)
    .join('; ');
  const fields = [
    ['author', paper.authors.map((author) => `{${bibtexValue(author.name)}}`).join(' and '), true],
    ['title', paper.title],
    ['year', year],
    ['doi', paper.doi],
    ['eprint', arxiv],
    ['archivePrefix', arxiv ? 'arXiv' : undefined],
    ['note', note],
    ['url', citationUrl(paper, origin, locator)],
  ] as const;
  return `@misc{${key},\n${fields
    .filter(([, value]) => Boolean(value))
    .map(([name, value, escaped]) => `  ${name} = {${escaped ? value : bibtexValue(value!)}},`)
    .join('\n')}\n}\n`;
}
