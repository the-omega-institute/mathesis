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
