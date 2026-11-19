import { afterEach, describe, expect, it, vi } from 'vitest';
import type { ApiClient } from '../api/client';
import { resolvePaperDoi } from './paperDoi';

afterEach(() => vi.unstubAllGlobals());
const signal = new AbortController().signal;
function api(papers: unknown[]) {
  return {
    listCollectedPapers: vi.fn().mockResolvedValue({ items: papers, next_before: null }),
    listPapers: vi.fn().mockResolvedValue({ items: [], next_before: null }),
    getCollectedPaper: vi.fn(),
  } as unknown as ApiClient;
}
describe('public DOI paper resolution', () => {
  it('resolves an older Zenodo DOI through its concept and the public version chain', async () => {
    const client = api([
      {
        id: 'paper',
        revision: '3',
        previous_version: '2',
        archive: { doi: '10.5281/zenodo.300', concept_doi: '10.5281/zenodo.100' },
      },
    ]);
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue(
        new Response(
          JSON.stringify({
            data: {
              attributes: {
                relatedIdentifiers: [
                  { relationType: 'IsVersionOf', relatedIdentifier: '10.5281/zenodo.100' },
                ],
              },
            },
          }),
        ),
      ),
    );
    vi.mocked(client.getCollectedPaper).mockResolvedValue({
      id: 'paper',
      revision: '2',
      archive: { doi: '10.5281/zenodo.200' },
    } as never);
    expect(await resolvePaperDoi(client, '10.5281/zenodo.200', signal)).toEqual({
      record: 'paper',
      version: '2',
    });
    expect(client.getCollectedPaper).toHaveBeenCalledWith('paper', '2', { signal });
  });
  it('resolves the exact archived paper version without interpreting the DOI suffix', async () => {
    const client = api([
      { id: 'different-title', revision: '2', archive: { doi: '10.5281/zenodo.24212412' } },
    ]);
    expect(await resolvePaperDoi(client, '10.5281/zenodo.24212412', signal)).toEqual({
      record: 'different-title',
      version: '2',
    });
    expect(client.listPapers).not.toHaveBeenCalled();
  });
  it('rechecks a historical catalog match against the public API before displaying it', async () => {
    const client = api([]);
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue(
        new Response(
          JSON.stringify({
            schema: 'mathesis.archive-index.v1',
            entries: [{ kind: 'collected', id: 'old-paper', version: 1, doi: '10.1000/old' }],
          }),
        ),
      ),
    );
    vi.mocked(client.getCollectedPaper).mockResolvedValue({
      id: 'old-paper',
      revision: '1',
      archive: { doi: '10.1000/wrong' },
    } as never);
    expect(await resolvePaperDoi(client, '10.1000/old', signal)).toBeNull();
    expect(client.getCollectedPaper).toHaveBeenCalledWith('old-paper', '1', { signal });
  });
  it('refuses an ambiguous DOI instead of choosing an arbitrary paper', async () => {
    const client = api([
      { id: 'one', archive: { doi: '10.1000/same' } },
      { id: 'two', archive: { doi: '10.1000/same' } },
    ]);
    await expect(resolvePaperDoi(client, '10.1000/same', signal)).rejects.toThrow('more than one');
  });
});
