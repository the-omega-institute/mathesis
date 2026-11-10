import snapshot from '../data/project-notes.json';
import type { Author, CollectedPaper, Settles } from '../api/types';
import type { PaperQuery } from '../api/client';
import { routePath } from '../routing/routes';
import { authorNameMatches, canonicalAuthorName } from './authorName';
import { noteSubjects } from './projectNoteSubjects';

export type ProjectNote = {
  archive?: { doi: string; concept_doi?: string };
  id: string;
  revision: string;
  title: string;
  abstract: string;
  summary: string;
  plain_language?: { en: string; zh: string };
  authors: Author[];
  license: string;
  published_on: string;
  problem_ids: string[];
  /** `id` is the lasting problem page of a live collected problem, when it has one. */
  problems: (Settles & { id?: string })[];
  lean?: { url: string; revision: string };
  pdf: { url: string; sha256: string };
  source: { url: string; sha256: string };
  review: { basis: string; source: string };
  msc?: string[];
};

/** Adapt a trusted API collection record to the same compact paper reader used by historical notes. */
export function collectedPaperNote(paper: CollectedPaper): ProjectNote {
  return {
    archive: paper.archive,
    id: paper.id,
    revision: paper.revision,
    title: paper.title,
    abstract: paper.abstract,
    summary: paper.plain_language?.en ?? paper.abstract,
    plain_language: paper.plain_language,
    authors: paper.authors,
    license: paper.license,
    published_on: paper.published_on,
    problem_ids: paper.problems.map((problem) => problem.source.locator),
    problems: paper.problems.map((problem, index) => ({
      ...problem,
      id: paper.problem_links?.[index]?.id,
    })),
    lean: paper.lean,
    pdf: { ...paper.pdf, sha256: '' },
    source: { ...paper.source, sha256: '' },
    review: { basis: 'trusted collection', source: 'published upstream review' },
    msc: paper.msc,
  };
}

/** The snapshot keeps each name as delivered; readers get the archive's one written form. */
export const projectNoteVersions: readonly ProjectNote[] = (snapshot.works as ProjectNote[]).map(
  (note) => ({
    ...note,
    authors: note.authors.map((a) => ({ ...a, name: canonicalAuthorName(a.name) })),
  }),
);
export const projectNotes = [...new Set(projectNoteVersions.map((note) => note.id))].map((id) =>
  projectNoteVersions
    .filter((note) => note.id === id)
    .reduce((latest, note) => (Number(note.revision) > Number(latest.revision) ? note : latest)),
);

/** Live versions replace migrated snapshots while each work keeps one archive entry. */
export function mergeCollectedNotes(
  papers: CollectedPaper[],
  historical: ProjectNote[] = projectNotes,
): ProjectNote[] {
  const notes = new Map(historical.map((note) => [note.id, note]));
  for (const paper of papers) {
    const previous = notes.get(paper.id);
    if (!previous || Number(paper.revision) >= Number(previous.revision)) {
      notes.set(paper.id, collectedPaperNote(paper));
    }
  }
  return [...notes.values()];
}

export function findProjectNotes(query: PaperQuery): ProjectNote[] {
  if (query.kind && query.kind !== 'note') return [];
  const msc = query.msc?.trim().toUpperCase();
  const q = (query.q ?? '').trim();
  const terms = q.toLocaleLowerCase().split(/\s+/).filter(Boolean);
  const author = (query.author ?? '').trim();
  return projectNotes.filter((note) => {
    const codes = note.msc ?? noteSubjects[note.id] ?? [];
    if (msc && !codes.some((code) => code.startsWith(msc))) return false;
    const authors = note.authors
      .map((a) => a.name)
      .join(' ')
      .toLocaleLowerCase();
    const text = [
      note.title,
      note.abstract,
      note.summary,
      note.archive?.doi ?? '',
      note.archive?.concept_doi ?? '',
      note.id,
      ...note.problem_ids,
      ...note.problems.map((p) => `${p.name} ${p.source.locator}`),
      authors,
    ]
      .join(' ')
      .toLocaleLowerCase();
    return (
      (!author || note.authors.some((a) => authorNameMatches(a.name, author))) &&
      // A whole query may also be one author's name in its other written form.
      (terms.every((term) => text.includes(term)) ||
        note.authors.some((a) => authorNameMatches(a.name, q)))
    );
  });
}

/** This is a citation to the source work, with no invented venue, DOI or verdict. */
export function projectCitation(note: ProjectNote): string {
  const doi = note.archive?.doi;
  return `${note.authors.map((a) => canonicalAuthorName(a.name)).join('; ')}. ${note.title}. Version ${note.revision}. ${note.published_on.slice(0, 4)}. ${doi ? `https://doi.org/${doi}` : `${window.location.origin}${routePath({ kind: 'paper', record: note.id, version: note.revision })}`}`;
}
