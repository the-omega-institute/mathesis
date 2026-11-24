import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { it, expect, vi } from 'vitest';
import { fakeApi, renderWithApp } from '../test/render';
import { FormalizationsPage } from './Formalizations';
import { SubmitPage } from './Submit';
import type { SharedFormalizationRequest } from '../api/types';
const url = 'https://github.com/the-omega-institute/trureturing-requests/issues/575';
const request: SharedFormalizationRequest = {
  number: 575,
  title: 'A sharp density bound',
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
  expect(await screen.findByRole('link', { name: request.title })).toHaveAttribute('href', url);
  expect(screen.getByRole('link', { name: 'contributor' })).toHaveAttribute(
    'href',
    'https://github.com/contributor',
  );
  expect(
    screen.getByText(
      'To claim a request, assign yourself or comment /claim. Comment /unclaim to release your claim.',
    ),
  ).toBeVisible();
  expect(old).not.toHaveBeenCalled();
  expect(screen.queryByRole('link', { name: 'Contribute a proof' })).not.toBeInTheDocument();
  const user = userEvent.setup();
  await user.selectOptions(screen.getByLabelText('Status'), 'merged');
  expect(await screen.findByText('No requests match these filters.')).toBeVisible();
});
it('keeps the canonical participation link available while synchronization is pending', async () => {
  renderWithApp(<FormalizationsPage view="list" />, fakeApi());
  expect(
    await screen.findByText(
      'The shared request list is being synchronized. You can participate directly in the shared queue.',
    ),
  ).toBeVisible();
  expect(screen.getByRole('link', { name: 'Open the shared request queue' })).toHaveAttribute(
    'href',
    url.slice(0, url.lastIndexOf('/')),
  );
});
it('lean entry uses shared requests publicly instead of creating another local proof job', async () => {
  const create = vi.fn();
  renderWithApp(<SubmitPage initialIntake="lean" />, fakeApi({ submitFormalization: create }));
  expect(await screen.findByRole('link', { name: 'Open the shared request queue' })).toBeVisible();
  expect(screen.queryByRole('form', { name: 'Submit Lean formalization' })).not.toBeInTheDocument();
  expect(create).not.toHaveBeenCalled();
});
