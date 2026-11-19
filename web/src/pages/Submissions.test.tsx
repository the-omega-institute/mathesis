import { screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, it, vi } from 'vitest';
import { author, submission, editor } from '../test/fixtures';
import { fakeApi, renderWithApp } from '../test/render';
import { SubmissionsPage } from './Submissions';
const delivered = {
  round: 3,
  version: 1,
  claims_revision: 1,
  delivered_at: '2026-10-27T21:00:00Z',
  in_progress: false,
  interrupted: false,
  has_execution_limits: false,
};
const paper = {
  ...submission,
  purpose: 'review_only' as const,
  status: { state: 'review_complete' as const },
};
it('loads only the small current summary and shows a delivered report', async () => {
  const reviewSummary = vi.fn(async () => delivered);
  const api = fakeApi(
    { listSubmissions: vi.fn(async () => ({ items: [paper], next_before: null })), reviewSummary },
    author,
  );
  renderWithApp(<SubmissionsPage scope="mine" />, api);
  expect(await screen.findByText('Report ready')).toBeVisible();
  expect(screen.queryByText(/Review progress unavailable/)).toBeNull();
  expect(api.referee).not.toHaveBeenCalled();
  expect(reviewSummary).toHaveBeenCalledOnce();
  window.dispatchEvent(new Event('focus'));
  expect(reviewSummary).toHaveBeenCalledOnce();
});
it('distinguishes loading and network failures from failed reviews, and supports retry', async () => {
  let reject!: (error: Error) => void;
  const reviewSummary = vi
    .fn()
    .mockImplementationOnce(
      () =>
        new Promise((_resolve, no) => {
          reject = no;
        }),
    )
    .mockResolvedValue(delivered);
  renderWithApp(
    <SubmissionsPage scope="mine" />,
    fakeApi(
      {
        listSubmissions: vi.fn(async () => ({ items: [paper], next_before: null })),
        reviewSummary,
      },
      author,
    ),
  );
  expect(await screen.findByText('Loading review status…')).toBeVisible();
  reject(new Error('Network unavailable'));
  expect(await screen.findByText('Review status could not be loaded')).toBeVisible();
  expect(screen.queryByText('Review interrupted')).toBeNull();
  await userEvent.click(screen.getByRole('button', { name: 'Retry status' }));
  expect(await screen.findByText('Report ready')).toBeVisible();
});
it('uses the staff route and qualifies delivered reports with limits', async () => {
  renderWithApp(
    <SubmissionsPage scope="queue" />,
    fakeApi(
      {
        listSubmissions: vi.fn(async () => ({ items: [paper], next_before: null })),
        reviewSummary: vi.fn(async () => ({ ...delivered, has_execution_limits: true })),
      },
      editor,
    ),
  );
  const row = (await screen.findByRole('link', { name: paper.title })).closest('li')!;
  expect(within(row).getByRole('link')).toHaveAttribute('href', `/staff/submissions/${paper.id}`);
  expect(await within(row).findByText('Report ready')).toBeVisible();
  expect(row).toHaveTextContent('checks remain incomplete');
});

it.each(['publication', 'review_only'] as const)(
  'keeps a delivered internal %s letter pending in My work',
  async (purpose) => {
    const reviewSummary = vi.fn(async () => delivered);
    renderWithApp(
      <SubmissionsPage scope="mine" />,
      fakeApi(
        {
          listSubmissions: vi.fn(async () => ({
            items: [{ ...submission, purpose }],
            next_before: null,
          })),
          reviewSummary,
        },
        author,
      ),
    );
    expect(
      await screen.findByText(purpose === 'review_only' ? 'In review' : 'Awaiting decision'),
    ).toBeVisible();
    expect(screen.queryByText('Report ready')).toBeNull();
    expect(screen.queryByText('The final report is ready to read.')).toBeNull();
    expect(screen.queryByText(/new results?/)).toBeNull();
  },
);

it('removes only the submitter’s withdrawn work and lets them restore it', async () => {
  const withdrawn = { ...submission, status: { state: 'withdrawn' as const } };
  const removed = { ...withdrawn, removed_from_work_at: '2026-11-19T07:00:00Z' };
  let isRemoved = false;
  const api = fakeApi(
    {
      listSubmissions: vi.fn(async (scope) => ({
        items: scope === 'removed' ? (isRemoved ? [removed] : []) : isRemoved ? [] : [withdrawn],
        next_before: null,
      })),
      removeSubmission: vi.fn(async () => {
        isRemoved = true;
        return removed;
      }),
      restoreSubmission: vi.fn(async () => {
        isRemoved = false;
        return withdrawn;
      }),
    },
    author,
  );
  renderWithApp(<SubmissionsPage scope="mine" />, api);
  await userEvent.click(await screen.findByRole('button', { name: 'Remove from my work' }));
  expect(await screen.findByText('You have not submitted work yet.')).toBeVisible();
  await userEvent.click(screen.getByRole('button', { name: 'Removed submissions' }));
  await userEvent.click(await screen.findByRole('button', { name: 'Restore to my work' }));
  expect(await screen.findByText('No removed submissions.')).toBeVisible();
  await userEvent.click(screen.getByRole('button', { name: 'Back to my work' }));
  expect(await screen.findByRole('link', { name: withdrawn.title })).toBeVisible();
  expect(api.removeSubmission).toHaveBeenCalledWith(submission.id);
  expect(api.restoreSubmission).toHaveBeenCalledWith(submission.id);
});
