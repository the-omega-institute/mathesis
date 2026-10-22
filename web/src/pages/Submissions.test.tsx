import { screen, within } from '@testing-library/react';
import { expect, it, vi } from 'vitest';
import { authorStanding } from '../lib/authorStanding';
import { authorStandingCases } from '../test/authorStanding';
import { author, submission } from '../test/fixtures';
import { queuedRound, refereeFile } from '../test/referee';
import { fakeApi, renderWithApp } from '../test/render';
import { SubmissionsPage } from './Submissions';

it('uses the same author standing and next action on a My work row', async () => {
  const review = refereeFile({ rounds: [queuedRound] });
  const api = fakeApi(
    {
      listSubmissions: vi.fn(async () => ({ items: [submission], next_before: null })),
      referee: vi.fn(async () => review),
    },
    author,
  );
  renderWithApp(<SubmissionsPage scope="mine" />, api);
  const title = await screen.findByRole('link', { name: submission.title });
  const row = title.closest('li')!;
  expect(await within(row).findByText(/Next action:/)).toBeVisible();
  const standing = authorStanding(submission, review);
  expect(row).toHaveTextContent(standing.nextAction);
  expect(row.querySelector('.author-next-action')).toHaveAttribute(
    'title',
    expect.stringContaining(standing.doing),
  );
  expect(api.referee).toHaveBeenCalledWith(submission.id, expect.any(Object));
  // List rows fetch once; a window focus must not start per-row polling.
  window.dispatchEvent(new Event('focus'));
  expect(api.referee).toHaveBeenCalledTimes(1);
  expect(row).toHaveTextContent('Paper');
  expect(row).not.toHaveTextContent('Private review · Private review');
});

it.each(authorStandingCases)(
  'keeps the My work badge and standing headline equal for $paper.status.state / $paper.kind',
  async ({ paper, review }) => {
    const api = fakeApi(
      {
        listSubmissions: vi.fn(async () => ({ items: [paper], next_before: null })),
        referee: vi.fn(async () => review),
      },
      author,
    );
    renderWithApp(<SubmissionsPage scope="mine" />, api);
    const row = (await screen.findByRole('link', { name: paper.title })).closest('li')!;
    const standing = authorStanding(paper, review);
    expect(await within(row).findByText(`${standing.headline} · Next action:`)).toBeVisible();
    expect(row.querySelector('.entry-meta .badge')).toHaveTextContent(standing.headline);
    expect(row.querySelector('.entry-meta .badge')?.textContent).toBe(standing.headline);
  },
);
