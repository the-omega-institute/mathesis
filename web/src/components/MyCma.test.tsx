import { screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { CmaAttemptView, MyCmaStatus } from '../api/types';
import { donor } from '../test/fixtures';
import { fakeApi, renderWithApp } from '../test/render';
import { CMA_COPY, CmaHistory, MyCma } from './MyCma';

const binding: MyCmaStatus = {
  enabled: true,
  setup_url: 'https://cma.example/auth/login?return_to=/',
  binding: {
    person: donor.id,
    subject: donor.id,
    revision: 2,
    generation: 1,
    status: 'setup_needed',
    reason: 'workspace_needed',
    workspace_id: null,
    sandbox_id: null,
    sandbox_profile: null,
    checked_at: null,
    created_at: '2026-10-11T00:00:00Z',
    updated_at: '2026-10-11T00:00:00Z',
  },
};
describe('Use your own agents (optional)', () => {
  it('explains the optional connection without loading CMA when signed out', async () => {
    const api = fakeApi();
    renderWithApp(<MyCma />, api);
    expect(await screen.findByText('Sign in to use your own agents.')).toBeInTheDocument();
    expect(
      screen.getByRole('heading', { name: 'Use your own agents (optional)' }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole('region', { name: 'Use your own agents (optional)' }),
    ).toHaveTextContent(CMA_COPY);
    expect(api.getCma).not.toHaveBeenCalled();
  });

  it('shows only the copy, status and Connect before consent', async () => {
    renderWithApp(
      <MyCma />,
      fakeApi(
        {
          getCma: vi.fn(async () => ({
            ...binding,
            binding: {
              ...binding.binding,
              status: 'disconnected' as const,
              reason: 'consent_pending',
            },
          })),
        },
        donor,
      ),
    );
    const panel = within(screen.getByRole('region', { name: 'Use your own agents (optional)' }));
    expect(await panel.findByText('Not connected')).toHaveAttribute('role', 'status');
    expect(panel.getAllByRole('link')).toHaveLength(1);
    expect(panel.getByRole('link', { name: 'Connect' })).toHaveAttribute(
      'href',
      expect.stringContaining('/auth/cma/connect?return_to='),
    );
    expect(panel.queryByRole('button')).toBeNull();
    expect(panel.queryByRole('form')).toBeNull();
    expect(panel.getAllByRole('paragraph')).toHaveLength(1);
    expect(panel.queryByText('Finish connecting through NyxID.')).toBeNull();
  });

  it('keeps disabled connections compact without offering a dead Connect link', async () => {
    renderWithApp(<MyCma />, fakeApi({}, donor));
    expect(await screen.findByText('Not available yet')).toHaveAttribute('role', 'status');
    expect(screen.queryByRole('link', { name: 'Connect' })).toBeNull();
    expect(screen.queryByLabelText('Compute workspace ID')).toBeNull();
  });

  it('validates setup with the binding revision, hides setup when ready and disconnects', async () => {
    const ready: MyCmaStatus = {
      ...binding,
      binding: {
        ...binding.binding,
        status: 'ready',
        revision: 3,
        reason: 'author_connected',
        workspace_id: 'wks_mine',
      },
    };
    const setCmaWorkspace = vi.fn(async () => ready);
    const disconnectCma = vi.fn(async () => ({
      ...binding,
      binding: {
        ...binding.binding,
        status: 'disconnected' as const,
        reason: 'not_connected',
        generation: 2,
        revision: 4,
      },
    }));
    renderWithApp(
      <MyCma />,
      fakeApi({ getCma: vi.fn(async () => binding), setCmaWorkspace, disconnectCma }, donor),
    );
    expect(await screen.findByText('Setup needed')).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Open compute setup' })).toHaveAttribute(
      'href',
      binding.setup_url,
    );
    expect(screen.getByRole('link', { name: 'Connect' })).toHaveAttribute(
      'href',
      expect.stringContaining('/auth/cma/connect?return_to='),
    );
    await userEvent.type(screen.getByLabelText('Compute workspace ID'), 'wks_mine');
    await userEvent.click(screen.getByRole('button', { name: 'Save / recheck workspace' }));
    expect(setCmaWorkspace).toHaveBeenLastCalledWith({
      workspace_id: 'wks_mine',
      expected_revision: 2,
    });
    expect(await screen.findByText('Connected')).toBeInTheDocument();
    expect(screen.queryByRole('form', { name: 'Compute workspace' })).toBeNull();
    expect(screen.queryByRole('link', { name: 'Open compute setup' })).toBeNull();
    expect(screen.queryByRole('link', { name: 'Connect' })).toBeNull();
    expect(screen.queryByText('Create a Compute workspace, then paste its ID below.')).toBeNull();
    await userEvent.click(screen.getByRole('button', { name: 'Disconnect' }));
    expect(disconnectCma).toHaveBeenCalledOnce();
    expect(await screen.findByText('Not connected')).toBeInTheDocument();
    expect(screen.queryByLabelText('Compute workspace ID')).toBeNull();
  });
  it('rechecks an unusable connection using the saved workspace and current revision', async () => {
    const unavailable: MyCmaStatus = {
      ...binding,
      binding: {
        ...binding.binding,
        status: 'unusable',
        reason: 'author_quota_refused',
        workspace_id: 'wks_existing',
        revision: 5,
      },
    };
    const setCmaWorkspace = vi.fn(async () => ({
      ...unavailable,
      binding: { ...unavailable.binding, status: 'ready' as const, reason: 'author_connected' },
    }));
    renderWithApp(
      <MyCma />,
      fakeApi({ getCma: vi.fn(async () => unavailable), setCmaWorkspace }, donor),
    );
    expect(await screen.findByText('Connection unavailable')).toBeInTheDocument();
    expect(
      screen.getByText('Restore your model quota in your compute account, then recheck.'),
    ).toBeInTheDocument();
    expect(screen.getByRole('form', { name: 'Compute workspace' })).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Save / recheck workspace' }));
    expect(setCmaWorkspace).toHaveBeenCalledWith({
      workspace_id: 'wks_existing',
      expected_revision: 5,
    });
    expect(await screen.findByText('Connected')).toBeInTheDocument();
    expect(screen.queryByLabelText('Compute workspace ID')).toBeNull();
  });
});

const attempt: CmaAttemptView = {
  kind: 'audit',
  step: 'r1:audit',
  attempt: 0,
  payer: 'operator',
  reason: 'not_connected',
  status: 'running',
  admitted_at: '2026-10-11T00:00:00Z',
  finished_at: null,
};

describe('Execution records', () => {
  it('shows compact rows and explains only the Mathesis fallback', async () => {
    renderWithApp(
      <CmaHistory
        attempts={[
          {
            ...attempt,
            payer: 'author',
            reason: 'author_connected',
            status: 'failed',
            admitted_at: '2026-10-11T00:00:00Z',
            finished_at: '2026-10-11T00:01:00Z',
          },
          {
            ...attempt,
            attempt: 1,
            payer: 'operator',
            reason: 'cause_unknown',
            status: 'running',
            admitted_at: '2026-10-11T00:01:00Z',
            finished_at: null,
          },
        ]}
      />,
      fakeApi(),
    );
    const history = screen.getByRole('region', { name: 'Execution records' });
    expect(within(history).getByRole('heading', { name: 'Execution records' })).toBeInTheDocument();
    expect(history.querySelector(':scope > p')).toHaveTextContent('Earlier failures');
    const list = within(history).getAllByRole('listitem');
    expect(list[0]).toHaveTextContent(/Source check · Your agents · Could not finish$/);
    expect(list[0]?.querySelector('p')).toBeNull();
    expect(list[1]).toHaveTextContent('Source check · Mathesis · In progress');
    expect(list[1]).toHaveTextContent('Your agents failed for an unknown reason.');
    expect(document.body).not.toHaveTextContent('workspace');
  });

  it.each(['account', 'auth', 'setup', 'credential', 'quota', 'cause_unknown'])(
    'explains a %s fallback',
    (reason) => {
      renderWithApp(<CmaHistory attempts={[{ ...attempt, reason }]} />, fakeApi());
      expect(screen.getByRole('listitem').querySelector('p')).toHaveTextContent(/Your/);
    },
  );

  it.each([
    ['operator', 'not_connected'],
    ['operator', 'byok_disabled'],
    ['operator', 'operator_step'],
    ['operator', 'legacy_unknown'],
    ['operator', 'workspace_needed'],
    ['operator', 'new_reason'],
    ['author', 'quota'],
  ] as const)('omits a reason for %s / %s without a fallback', (payer, reason) => {
    renderWithApp(<CmaHistory attempts={[{ ...attempt, payer, reason }]} />, fakeApi());
    expect(screen.getByRole('listitem').querySelector('p')).toBeNull();
  });

  it('keeps a step without a kind readable and hides an empty history', () => {
    const { rerender } = renderWithApp(
      <CmaHistory attempts={[{ ...attempt, kind: undefined }]} />,
      fakeApi(),
    );
    expect(screen.getByRole('listitem')).toHaveTextContent(
      /Earlier review step · Mathesis · In progress$/,
    );
    rerender(<CmaHistory attempts={[]} />);
    expect(screen.queryByRole('region', { name: 'Execution records' })).toBeNull();
  });
  it('gives an unknown legacy step kind a readable fallback', () => {
    const legacyAttempt = JSON.parse(JSON.stringify({ ...attempt, kind: 'old_automated_step' }));
    renderWithApp(<CmaHistory attempts={[legacyAttempt]} />, fakeApi());
    expect(screen.getByRole('listitem')).toHaveTextContent(
      /Other automated step · Mathesis · In progress$/,
    );
    expect(document.body).not.toHaveTextContent('old_automated_step');
  });
});
