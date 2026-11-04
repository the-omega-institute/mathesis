import snapshot from '../data/project-notes.json';
import type { Author, Settles } from '../api/types';
import type { PaperQuery } from '../api/client';
import { routePath } from '../routing/routes';

export type ProjectNote = {
  id: string;
  revision: string;
  title: string;
  abstract: string;
  summary: string;
  authors: Author[];
  license: string;
  published_on: string;
  problem_ids: string[];
  problems: Settles[];
  lean: { url: string; revision: string };
  pdf: { url: string; sha256: string };
  source: { url: string; sha256: string };
  review: { basis: string; source: string };
};

export const projectNoteVersions: readonly ProjectNote[] = snapshot.works as ProjectNote[];
export const projectNotes = [...new Set(projectNoteVersions.map((note) => note.id))].map((id) =>
  projectNoteVersions
    .filter((note) => note.id === id)
    .reduce((latest, note) => (Number(note.revision) > Number(latest.revision) ? note : latest)),
);

export function findProjectNotes(query: PaperQuery): ProjectNote[] {
  if (query.msc || (query.kind && query.kind !== 'note')) return [];
  const terms = (query.q ?? '').trim().toLocaleLowerCase().split(/\s+/).filter(Boolean);
  const author = (query.author ?? '').trim().toLocaleLowerCase();
  return projectNotes.filter((note) => {
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
