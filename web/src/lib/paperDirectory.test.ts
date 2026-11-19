import { describe, expect, it, vi } from 'vitest';
import type { CollectedPaper } from '../api/types';
import { fakeApi } from '../test/render';
import { listDirectoryNotes } from './paperDirectory';
import {
  collectedPaperNote,
  findProjectNotes,
  mergeCollectedNotes,
  projectNotes,
} from './projectNotes';
import { publicationSubjects } from './projectNoteSubjects';

function live(id: string, msc: string[] = []): CollectedPaper {
  const note = projectNotes[0]!;
  return {
    ...note,
    id,
    msc,
    pdf: { ...note.pdf, bytes: 1 },
    source: { ...note.source, bytes: 1 },
  };
}

describe('paper directory subjects', () => {
  it('finds legacy papers beyond the first collection page without dropping search filters', async () => {
    const first = live('abbott-2019-sequential-weak-pointer-bound');
    const second = live('a320043-square-values-refutation');
    const listCollectedPapers = vi.fn(async (query) =>
      query.before
        ? { items: [second], next_before: null }
        : { items: [first], next_before: 'page-2' },
    );
    const signal = new AbortController().signal;
    const page = await listDirectoryNotes(
      fakeApi({ listCollectedPapers }),
      { msc: '11', author: 'Haobo Ma' },
      signal,
    );
    expect(listCollectedPapers.mock.calls).toEqual([
      [{ author: 'Haobo Ma', before: null, limit: 100 }, { signal }],
      [{ author: 'Haobo Ma', before: 'page-2', limit: 100 }, { signal }],
    ]);
    expect(
      findProjectNotes({ msc: '11' }, page.items.map(collectedPaperNote)).map((note) => note.id),
    ).toEqual([second.id]);
  });

  it('uses supplied nonempty MSC ahead of editorial labels and superseded snapshots', () => {
    const old = projectNotes.find((note) => note.id === 'b164660-chebfloor')!;
    const successor = { ...live(old.id, ['81Q10']), revision: String(Number(old.revision) + 1) };
    const merged = mergeCollectedNotes([successor], [old]);
    expect(findProjectNotes({ msc: '11' }, merged)).toEqual([]);
    expect(findProjectNotes({ msc: '81Q' }, merged)).toHaveLength(1);
    expect(publicationSubjects('a320043-square-values-refutation', [])).toEqual(['11']);
    expect(publicationSubjects('unknown-paper', [])).toEqual([]);
  });

  it('surfaces incomplete loads and stops cancelled requests', async () => {
    const repeated = fakeApi({
      listCollectedPapers: vi.fn(async () => ({ items: [], next_before: 'same' })),
    });
    await expect(listDirectoryNotes(repeated, {}, new AbortController().signal)).rejects.toThrow(
      'could not finish loading',
    );
    expect(repeated.listCollectedPapers).toHaveBeenCalledTimes(2);
    const controller = new AbortController();
    const api = fakeApi({
      listCollectedPapers: vi.fn(async () => {
        controller.abort();
        return { items: [], next_before: 'next' };
      }),
    });
    await expect(listDirectoryNotes(api, {}, controller.signal)).rejects.toThrow();
    expect(api.listCollectedPapers).toHaveBeenCalledTimes(1);
  });
});
