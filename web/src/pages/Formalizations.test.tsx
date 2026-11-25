import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { it, expect, vi } from 'vitest';
import { SharedRequestLeaderboard } from '../components/SharedRequests';
import { fakeApi, renderWithApp } from '../test/render';
import { FormalizationsPage } from './Formalizations';
import { SubmitPage } from './Submit';
import type { SharedFormalizationRequest } from '../api/types';
const url = 'https://github.com/the-omega-institute/trureturing-requests/issues/575';
const request: SharedFormalizationRequest = {
  number: 575,
  title: 'A sharp density bound',
  body: 'The target is **available in Mathesis**.\n\n$$d(x,y) \\leq 1$$\n\n<!-- hidden-source-marker -->',
  comments: [
    {
      id: 123,
      author: { login: 'contributor', url: 'https://github.com/contributor', agent: false },
      body: 'I have checked the **boundary case**.',
      created_at: '2026-11-24T01:00:00Z',
      updated_at: '2026-11-24T01:00:00Z',
    },
  ],
  url,
  state: 'claimed',
  updated_at: '2026-11-24T00:00:00Z',
  requested_by: 'researcher',
  claimants: [{ login: 'contributor', url: 'https://github.com/contributor', agent: false }],
  completions: [],
  paper_repository: 'https://github.com/mathesis-papers/paper',
  lean_source: null,
};
it('takes requests and their claimants from the shared queue, without calling the legacy target pool', async () => {
  const old = vi.fn();
  renderWithApp(
    <FormalizationsPage view="list" />,
    fakeApi({
      formalizationOpportunities: old,
      sharedFormalizationRequests: async () => ({
        revision: 1,
        observed_at: '2026-11-24T00:00:00Z',
        requests: [request],
      }),
    }),
  );
  expect(await screen.findByRole('link', { name: /A sharp density bound/ })).toHaveAttribute(
    'href',
    '/formalizations?view=list&request=575',
  );
  const user = userEvent.setup();
  await user.click(screen.getByRole('link', { name: /A sharp density bound/ }));
  expect(screen.getByText('available in Mathesis')).toBeVisible();
  expect(screen.getAllByRole('link', { name: 'contributor' })[0]).toHaveAttribute(
    'href',
    'https://github.com/contributor',
  );
  expect(screen.getByText('boundary case')).toBeVisible();
  expect(screen.getByRole('link', { name: 'Original on GitHub' })).toHaveAttribute('href', url);
  expect(document.querySelector('.katex')).not.toBeNull();
  expect(document.body).not.toHaveTextContent('hidden-source-marker');
  await user.click(screen.getByRole('link', { name: 'Back to awaiting formalization' }));
  expect(old).not.toHaveBeenCalled();
  expect(screen.queryByRole('link', { name: 'Contribute a proof' })).not.toBeInTheDocument();
  await user.selectOptions(screen.getByLabelText('Status'), 'merged');
  expect(await screen.findByText('No requests match these filters.')).toBeVisible();
});
it('keeps pending synchronization inside Mathesis', async () => {
  renderWithApp(<FormalizationsPage view="list" />, fakeApi());
  expect(
    await screen.findByText(
      'Formalization requests are being synchronized. Please check again shortly.',
    ),
  ).toBeVisible();
  expect(
    screen.queryByRole('link', { name: 'Open the shared request queue' }),
  ).not.toBeInTheDocument();
});
it('lean entry uses shared requests publicly instead of creating another local proof job', async () => {
  const create = vi.fn();
  renderWithApp(<SubmitPage initialIntake="lean" />, fakeApi({ submitFormalization: create }));
  expect(
    await screen.findByText(
      'Formalization requests are being synchronized. Please check again shortly.',
    ),
  ).toBeVisible();
  expect(
    screen.queryByRole('link', { name: 'Open the shared request queue' }),
  ).not.toBeInTheDocument();
  expect(screen.queryByRole('form', { name: 'Submit Lean formalization' })).not.toBeInTheDocument();
  expect(create).not.toHaveBeenCalled();
});

it('links leaderboard requests to their in-site reader', () => {
  renderWithApp(
    <SharedRequestLeaderboard
      rows={[{ rank: 1, contributor: request.claimants[0]!, completed: 1, requests: [575] }]}
    />,
    fakeApi(),
  );
  expect(screen.getByRole('link', { name: '#575' })).toHaveAttribute(
    'href',
    '/formalizations?view=list&request=575',
  );
});

it('opens a completed request from a direct link even though the default list is open work', async () => {
  window.history.replaceState(null, '', '/formalizations?view=list&request=575');
  renderWithApp(
    <FormalizationsPage view="list" />,
    fakeApi({
      sharedFormalizationRequests: async () => ({
        revision: 1,
        observed_at: request.updated_at,
        requests: [{ ...request, state: 'merged' }],
      }),
    }),
  );
  expect(await screen.findByText('available in Mathesis')).toBeVisible();
  expect(screen.getByText('boundary case')).toBeVisible();
});

it('keeps a legacy request readable while its description is backfilled', async () => {
  window.history.replaceState(null, '', '/formalizations?view=list&request=575');
  renderWithApp(
    <FormalizationsPage view="list" />,
    fakeApi({
      sharedFormalizationRequests: async () => ({
        revision: 1,
        observed_at: request.updated_at,
        requests: [{ ...request, body: undefined, comments: undefined }],
      }),
    }),
  );
  expect(await screen.findByText('The request description is being synchronized.')).toBeVisible();
});
