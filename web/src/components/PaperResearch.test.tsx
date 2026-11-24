import { screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { it, expect, vi } from 'vitest';
import type { PaperResearch as Research } from '../api/types';
import { fakeApi, renderWithApp } from '../test/render';
import { PaperResearch } from './PaperResearch';

const reading: Research = {
  work: 'a-note',
  version: 2,
  problems: [],
  macros: {},
  lean_source: 'https://github.com/example/paper/blob/main/Proof.lean',
  requests_observed_at: '2026-11-24T00:00:00Z',
  results: [
    {
      claim: 'C1',
      name: 'A sharp bound',
      kind: 'theorem',
      role: 'main',
      section: null,
      statement: 'The bound holds for every positive integer.',
      state: 'published',
    },
  ],
  questions: [
    {
      claim: 'C2',
      name: 'Extension to all dimensions',
      kind: 'conjecture',
      role: null,
      section: null,
      statement: 'Does it hold in every dimension?',
      state: 'published',
    },
  ],
  requests: [
    {
      association: 'repository',
      request: {
        number: 5,
        title: 'Formalize the finite case',
        state: 'merged',
        url: 'https://github.com/the-omega-institute/trureturing-requests/issues/5',
        updated_at: '2026-11-24T00:00:00Z',
        requested_by: 'author',
        claimants: [],
        lean_source: null,
        paper_repository: 'https://github.com/example/paper',
        completions: [
          {
            url: 'https://github.com/the-omega-institute/trureturing/pull/5',
            revision: 'a'.repeat(40),
            merged_at: '2026-11-24T00:00:00Z',
            contributor: {
              login: 'Contributor',
              url: 'https://github.com/Contributor',
              agent: false,
            },
            ranked: false,
          },
        ],
      },
    },
  ],
};
it('shows named mathematics and scoped shared contributions without inventing paper verification', async () => {
  const api = fakeApi({ paperResearch: vi.fn(async () => reading) });
  renderWithApp(
    <PaperResearch
      work="a-note"
      version={2}
      origin="https://mathesis.example/papers/doi/10.1234/example"
    />,
    api,
  );
  const result = await screen.findByRole('link', { name: 'A sharp bound' });
  expect(result).toHaveAttribute('href', '/papers/a-note/results/C1?version=2');
  expect(screen.queryByText('Verified in Lean')).toBeNull();
  expect(document.body).not.toHaveTextContent('C1');
  await userEvent.click(within(result.closest('li')!).getByText('Read the statement'));
  expect(screen.getByText('The bound holds for every positive integer.')).toBeVisible();
  expect(screen.getByRole('link', { name: 'Extension to all dimensions' })).toHaveAttribute(
    'href',
    '/papers/a-note/results/C2?version=2',
  );
  expect(screen.getByRole('heading', { name: 'Related formalization work' })).toBeVisible();
  expect(screen.getByRole('link', { name: 'Read the contribution' })).toHaveAttribute(
    'href',
    reading.requests[0]!.request.completions[0]!.url,
  );
  expect(screen.getByRole('link', { name: 'Contributor' })).toBeVisible();
  const action = new URL(
    screen.getByRole('link', { name: 'Propose a further question' }).getAttribute('href')!,
    window.location.origin,
  );
  expect(action.searchParams.get('type')).toBe('conjecture');
  expect(action.searchParams.get('origin')).toBe(
    'https://mathesis.example/papers/doi/10.1234/example',
  );
  expect(api.paperResearch).toHaveBeenCalledWith(
    'a-note',
    2,
    expect.objectContaining({ signal: expect.any(AbortSignal) }),
  );
});
it('keeps the existing reading and declared request links available if enrichment is unavailable', async () => {
  renderWithApp(
    <PaperResearch
      work="a-note"
      version={2}
      origin="source"
      fallback={<p>Existing public results</p>}
      materials={{
        version: 2,
        repository_url: 'https://github.com/example/paper',
        site_url: 'https://example.test',
        reading: {},
        slides: [],
        visualizations: [],
        formalization_requests: [{ label: 'An exact task', url: reading.requests[0]!.request.url }],
      }}
    />,
    fakeApi(),
  );
  expect(await screen.findByText('Existing public results')).toBeVisible();
  expect(screen.getByRole('link', { name: 'An exact task' })).toHaveAttribute(
    'href',
    reading.requests[0]!.request.url,
  );
  expect(screen.queryByText('Merged contribution')).toBeNull();
});
it('does not attach another paper or version to this manuscript', async () => {
  const api = fakeApi({ paperResearch: vi.fn(async () => ({ ...reading, version: 3 })) });
  renderWithApp(
    <PaperResearch
      work="a-note"
      version={2}
      origin="source"
      fallback={<p>Existing public results</p>}
    />,
    api,
  );
  await screen.findByText('Existing public results');
  expect(screen.queryByRole('link', { name: 'A sharp bound' })).toBeNull();
  expect(screen.queryByRole('link', { name: 'Read the contribution' })).toBeNull();
});

it('keeps public questions separate from results when there is no proved source result', async () => {
  renderWithApp(
    <PaperResearch
      work="a-note"
      version={2}
      origin="source"
      fallback={<p>Old statement list</p>}
    />,
    fakeApi({ paperResearch: vi.fn(async () => ({ ...reading, results: [], requests: [] })) }),
  );
  await screen.findByRole('link', { name: 'Extension to all dimensions' });
  expect(screen.queryByText('Old statement list')).toBeNull();
  expect(screen.queryByRole('heading', { name: 'Results' })).toBeNull();
});
