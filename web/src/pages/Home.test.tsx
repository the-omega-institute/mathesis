import { screen, within } from '@testing-library/react';
import { expect, it, vi } from 'vitest';
import { fakeApi, renderWithApp } from '../test/render';
import { HomePage } from './Home';

it('shows recent shared proof work with its contributor alongside research without inventing verification', async () => {
  renderWithApp(
    <HomePage />,
    fakeApi({
      sharedFormalizationRequests: vi.fn(async () => ({
        revision: 1,
        observed_at: '2026-11-24T00:00:00Z',
        requests: [
          {
            number: 5,
            title: 'A proof of the finite bound',
            url: 'https://github.com/the-omega-institute/trureturing-requests/issues/5',
            state: 'merged' as const,
            updated_at: '2026-11-24T00:00:00Z',
            requested_by: 'author',
            claimants: [],
            paper_repository: null,
            lean_source: null,
            completions: [
              {
                url: 'https://github.com/the-omega-institute/trureturing/pull/5',
                revision: 'a'.repeat(40),
                merged_at: '2026-11-24T00:00:00Z',
                contributor: {
                  login: 'A Contributor',
                  url: 'https://github.com/contributor',
                  agent: false,
                },
                ranked: false,
              },
            ],
          },
        ],
      })),
    }),
  );
  const section = screen.getByRole('region', { name: 'Latest research' });
  const link = await within(section).findByRole('link', { name: 'A proof of the finite bound' });
  expect(link).toHaveAttribute(
    'href',
    'https://github.com/the-omega-institute/trureturing-requests/issues/5',
  );
  expect(within(section).getByRole('link', { name: 'A Contributor' })).toBeVisible();
  expect(within(section).getAllByRole('link', { name: 'Read PDF' })).toHaveLength(2);
  expect(within(section).queryByText('Verified in Lean')).toBeNull();
  expect(screen.getByRole('link', { name: 'Submit a paper' })).toHaveAttribute('href', '/submit');
  expect(screen.getByRole('link', { name: 'Agent submission guide' })).toHaveAttribute(
    'href',
    '/skill.md',
  );
});
