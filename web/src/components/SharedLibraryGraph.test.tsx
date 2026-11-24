import { beforeEach, expect, it, vi } from 'vitest';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { fakeApi, renderWithApp } from '../test/render';
import type { CollectedPaper } from '../api/types';
import type { SharedLibrary } from '../lib/sharedLibrary';
import { loadSharedLibrary } from '../lib/sharedLibrary';
import { SharedLibraryGraph } from './SharedLibraryGraph';
import { FormalizationsPage } from '../pages/Formalizations';

vi.mock('../lib/sharedLibrary', async (original) => ({
  ...(await original<typeof import('../lib/sharedLibrary')>()),
  loadSharedLibrary: vi.fn(),
}));
const commit = 'b'.repeat(40);
const source = (module: string) =>
  `https://github.com/the-omega-institute/trureturing/blob/${commit}/${module}.lean`;
const papers = [
  {
    id: 'quantum-paper',
    revision: '1',
    title: 'Projection bounds',
    msc: ['81P15'],
    lean: { url: source('Quantum/Bound'), revision: commit },
  },
  {
    id: 'integer-paper',
    revision: '1',
    title: 'Integer identities',
    msc: ['11A41'],
    lean: { url: source('Arith/Identity'), revision: commit },
  },
] as CollectedPaper[];
const library: SharedLibrary = {
  sourceCommit: commit,
  digest: 'sha256:' + 'a'.repeat(64),
  nodes: ['Quantum/Bound', 'Arith/Identity', 'Algebra/Base'].map((id) => ({
    id,
    title: id,
    abstract: '',
    domain: id.split('/')[0]!,
    source: source(id),
    formalized: true,
    x: 0,
    y: 0,
  })),
  edges: [{ from: 'Algebra/Base', to: 'Quantum/Bound' }],
};
beforeEach(() => {
  window.history.replaceState(null, '', '/formalizations');
  vi.mocked(loadSharedLibrary).mockReset().mockResolvedValue(library);
  vi.stubGlobal(
    'ResizeObserver',
    class {
      observe() {}
      disconnect() {}
    },
  );
  vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue(null);
});
it('opens the existing knowledge graph independently of the paper archive', async () => {
  const api = fakeApi({
    listCollectedPapers: vi.fn(async () => ({ items: papers, next_before: null })),
  });
  renderWithApp(<FormalizationsPage focus="library:all" />, api);
  expect(await screen.findByRole('heading', { name: 'Number theory' })).toBeInTheDocument();
  expect(screen.getByRole('img', { name: /Mathematical dependency graph/ })).toBeInTheDocument();
  expect(screen.getByRole('option', { name: 'Quantum theory' })).toBeInTheDocument();
  expect(loadSharedLibrary).toHaveBeenCalled();
  expect(api.listCollectedPapers).not.toHaveBeenCalled();
  expect(api.collectedKnowledge).not.toHaveBeenCalled();
  expect(screen.queryByText('Papers in this graph')).not.toBeInTheDocument();
});
it('loads only the selected subject and keeps a cross-subject prerequisite inside that graph', async () => {
  const api = fakeApi({
    listCollectedPapers: vi.fn(async () => ({ items: papers, next_before: null })),
  });
  renderWithApp(<SharedLibraryGraph msc="81" focus="Quantum/Bound" />, api);
  expect(await screen.findByRole('heading', { name: 'Quantum theory' })).toBeInTheDocument();
  expect(api.collectedKnowledge).not.toHaveBeenCalled();
  expect(api.listCollectedPapers).not.toHaveBeenCalled();
  expect(screen.getByRole('heading', { name: 'Builds on (1)' })).toBeInTheDocument();
  await userEvent.click(screen.getByRole('button', { name: 'Algebra/Base' }));
  expect(screen.getByText('Prerequisite from another subject')).toBeInTheDocument();
  expect(screen.getByRole('heading', { name: 'Quantum theory' })).toBeInTheDocument();
  expect(screen.queryByText('Integer identities')).not.toBeInTheDocument();
  await userEvent.selectOptions(screen.getByRole('combobox', { name: 'Subject' }), '11');
  await waitFor(() => expect(window.location.search).toBe('?msc=11'));
});
it('preserves old paper-to-module links by resolving them into the matching subject URL', async () => {
  const api = fakeApi({
    listCollectedPapers: vi.fn(async () => ({ items: papers, next_before: null })),
  });
  renderWithApp(<SharedLibraryGraph focus="Quantum/Bound" />, api);
  await waitFor(() => expect(new URLSearchParams(window.location.search).get('msc')).toBe('81'));
  expect(new URLSearchParams(window.location.search).get('focus')).toBe('module:Quantum/Bound');
});

it('keeps the published library available when the shared request feed is unavailable', async () => {
  const api = fakeApi({
    sharedFormalizationRequests: vi.fn(async () => {
      throw new Error('temporarily unavailable');
    }),
  });
  renderWithApp(<SharedLibraryGraph msc="81" />, api);
  expect(await screen.findByRole('heading', { name: 'Quantum theory' })).toBeInTheDocument();
  expect(screen.getByRole('img', { name: /Mathematical dependency graph/ })).toBeInTheDocument();
  expect(api.sharedFormalizationRequests).toHaveBeenCalled();
});
