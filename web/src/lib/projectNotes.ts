import snapshot from '../data/project-notes.json';
import type { Author, CollectedPaper, Settles } from '../api/types';
import type { PaperQuery } from '../api/client';
import { routePath } from '../routing/routes';
import { noteSubjects } from './projectNoteSubjects';

export type ProjectNote = {
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
  problems: Settles[];
  lean?: { url: string; revision: string };
  pdf: { url: string; sha256: string };
  source: { url: string; sha256: string };
  review: { basis: string; source: string };
  msc?: string[];
};

/** Adapt a trusted API collection record to the same compact paper reader used by historical notes. */
export function collectedPaperNote(paper: CollectedPaper): ProjectNote {
  return {
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
    problems: paper.problems,
    lean: paper.lean,
    pdf: { ...paper.pdf, sha256: '' },
    source: { ...paper.source, sha256: '' },
    review: { basis: 'trusted collection', source: 'published upstream review' },
    msc: paper.msc,
  };
}

export const projectNoteVersions: readonly ProjectNote[] = snapshot.works as ProjectNote[];
export const projectNotes = [...new Set(projectNoteVersions.map((note) => note.id))].map((id) =>
  projectNoteVersions
    .filter((note) => note.id === id)
    .reduce((latest, note) => (Number(note.revision) > Number(latest.revision) ? note : latest)),
);

export function findProjectNotes(query: PaperQuery): ProjectNote[] {
  if (query.kind && query.kind !== 'note') return [];
  const msc = query.msc?.trim().toUpperCase();
  const terms = (query.q ?? '').trim().toLocaleLowerCase().split(/\s+/).filter(Boolean);
  const author = (query.author ?? '').trim().toLocaleLowerCase();
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
      note.id,
      ...note.problem_ids,
      ...note.problems.map((p) => `${p.name} ${p.source.locator}`),
      authors,
    ]
      .join(' ')
      .toLocaleLowerCase();
    return (!author || authors.includes(author)) && terms.every((term) => text.includes(term));
  });
}

/** This is a citation to the source work, with no invented venue, DOI or verdict. */
export function projectCitation(note: ProjectNote): string {
  return `${note.authors.map((a) => a.name).join('; ')}. ${note.title}. Version ${note.revision}. ${note.published_on.slice(0, 4)}. ${window.location.origin}${routePath({ kind: 'paper', record: note.id, version: note.revision })}`;
}
