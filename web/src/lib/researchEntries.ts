import type { PaperSummary } from '../api/types';
import type { ProjectNote } from './projectNotes';

export type ResearchEntry =
  { kind: 'paper'; paper: PaperSummary } | { kind: 'note'; note: ProjectNote };

/** Published manuscripts share a bibliography regardless of their intake path. */
export function researchEntries(papers: PaperSummary[], notes: ProjectNote[]): ResearchEntry[] {
  const entries: ResearchEntry[] = [
    ...papers.map((paper) => ({ kind: 'paper' as const, paper })),
    ...notes.map((note) => ({ kind: 'note' as const, note })),
  ];
  const date = (entry: ResearchEntry) =>
    entry.kind === 'paper' ? (entry.paper.accepted_at ?? '') : entry.note.published_on;
  return entries.sort((a, b) => date(b).localeCompare(date(a)));
}
