import { readFileSync } from 'node:fs';
import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { AttemptView, ConjectureDetail, Entrant, VerificationReceipt } from '../api/types';
import { App } from '../App';
import { author, paperSummary } from '../test/fixtures';
import { fakeApi, renderWithApp } from '../test/render';
import { ConjecturePage } from './Conjecture';
import { ConjecturesPage } from './Conjectures';
import { HomePage } from './Home';
import { EntrantPage, LeaderboardPage } from './Leaderboard';

const agent: Entrant = {
  id: 'agent:proof',
  name: 'Proof agent',
  kind: 'agent',
  owner: { id: author.id, name: author.display_name },
  retired: false,
  revision: 0,
};
const receipt: VerificationReceipt = {
  verdict: 'proved',
  reason: 'Lean checked the target',
  target_digest: 't'.repeat(64),
  solution_digest: 's'.repeat(64),
  toolchain: 'lean, Mathlib revision',
  axioms: ['propext'],
  checked_at: '2026-10-11T00:00:00Z',
  duration: 25,
};
const detail: ConjectureDetail = {
  summary: {
    claim: 'C2',
    source: 'from WP-2026-0001',
    status: 'open',
    attempts: 0,
    solver: null,
    record: 'WP-2026-0001',
    title: 'An open bound',
    statement: 'For all $n$, $n = n$.',
    lean_statement_status: 'confirmed',
  },
  macros: {},
  target: {
    claim: 'C2',
    lean: 'import Mathlib\ndef wishpool_target_prop : Prop := True\n',
    digest: receipt.target_digest,
    toolchain: receipt.toolchain,
  },
  verified_attempts: [],
};
const queued: AttemptView = {
  id: 'attempt-one',
  record: detail.summary.record,
  claim: 'C2',
  entrant: agent,
  state: 'queued',
  receipt: null,
  solution: 'import Target\ntheorem wishpool_solution : wishpool_target_prop := by trivial',
  note: 'Only the owner sees this',
};
const row = {
  rank: 1,
  entrant: agent,
  solved: 1,
  disproved: 0,
  score: 1,
  others: 1,
  total: 1,
  last_solve: receipt.checked_at,
};

describe('solving pages', () => {
  it('renders the confirmed target for anonymous readers and requires sign-in to upload', async () => {
    const api = fakeApi({ getConjecture: vi.fn(async () => detail) });
    renderWithApp(<ConjecturePage record={detail.summary.record} claim="C2" />, api);
    expect(await screen.findByRole('heading', { name: 'Open' })).toBeInTheDocument();
    expect(screen.getByText(detail.summary.source, { exact: false })).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Download Target.lean' })).toHaveAttribute(
      'download',
      'Target.lean',
    );
    expect(screen.getByText('View Target.lean').closest('details')).toHaveAttribute('open');
    expect(screen.getByText(/Sign in to submit a Lean solution/)).toBeInTheDocument();
    expect(screen.queryByLabelText('Note (optional, private)')).not.toBeInTheDocument();
    expect(screen.getByText(/mathesis-contribute attempt WP-2026-0001 C2/)).toBeInTheDocument();
    expect(document.querySelector('.katex')).not.toBeNull();
  });

  it('uploads a solution as an owned agent, keeps the note private and refreshes the verifier status', async () => {
    const user = userEvent.setup();
    const submitAttempt = vi.fn(async () => queued);
    const getAttempt = vi.fn(async () => ({
      ...queued,
      state: 'rejected' as const,
      receipt: { ...receipt, verdict: 'rejected' as const, reason: 'Wrong target type' },
    }));
    const api = fakeApi(
      {
        getConjecture: vi.fn(async () => detail),
        agents: vi.fn(async () => [
          agent,
          { ...agent, id: 'retired', name: 'Retired agent', retired: true },
        ]),
        submitAttempt,
        getAttempt,
      },
      author,
    );
    renderWithApp(<ConjecturePage record={detail.summary.record} claim="C2" />, api);
    await screen.findByRole('option', { name: 'Proof agent (Agent)' });
    expect(screen.queryByRole('option', { name: 'Retired agent (Agent)' })).not.toBeInTheDocument();
    const file = new File([queued.solution], 'Solution.lean', { type: 'text/plain' });
    Object.defineProperty(file, 'text', { value: async () => queued.solution });
    await user.upload(screen.getByLabelText('Lean solution (up to 1 MB)'), file);
    await user.selectOptions(screen.getByLabelText('Submit as'), agent.id);
    await user.type(screen.getByLabelText('Note (optional, private)'), queued.note!);
    await user.click(screen.getByRole('button', { name: 'Submit for verification' }));
    expect(await screen.findByText('Attempt attempt-one: queued')).toBeInTheDocument();
    expect(submitAttempt).toHaveBeenCalledWith(
      detail.summary.record,
      'C2',
      queued.solution,
      agent.id,
      queued.note,
    );
    expect(screen.getByText(/Your attempt is private/)).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Refresh status' }));
    expect(await screen.findByText('Wrong target type')).toBeInTheDocument();
    expect(getAttempt).toHaveBeenCalledWith(queued.id);
  });

  it('shows the winning solution, receipt and also-verified attempts without a private note', async () => {
    const solved: ConjectureDetail = {
      ...detail,
      summary: { ...detail.summary, status: 'solved', solver: agent, attempts: 2 },
      verified_attempts: [
        { id: 'first', entrant: agent, receipt, solution: queued.solution, also_verified: false },
        { id: 'second', entrant: agent, receipt, solution: queued.solution, also_verified: true },
      ],
    };
    renderWithApp(
      <ConjecturePage record={detail.summary.record} claim="C2" />,
      fakeApi({ getConjecture: vi.fn(async () => solved) }),
    );
    expect(
      await screen.findByRole('heading', { name: /Solved by Proof agent/ }),
    ).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Verified solution' })).toBeInTheDocument();
    expect(screen.getByText('Verification receipt')).toBeInTheDocument();
    expect(screen.getByText(/Also verified/)).toBeInTheDocument();
    expect(screen.queryByText(queued.note!)).not.toBeInTheDocument();
    expect(screen.getAllByText('Agent').length).toBeGreaterThan(0);
  });

  it('requests status-filtered conjectures and gives a submit link for an empty filter', async () => {
    const user = userEvent.setup();
    const listConjectures = vi.fn(async () => ({ items: [], next_before: null }));
    renderWithApp(<ConjecturesPage />, fakeApi({ listConjectures }));
    await screen.findByText('No open conjectures here yet.');
    await user.click(screen.getByRole('button', { name: 'Disproved' }));
    await waitFor(() =>
      expect(listConjectures).toHaveBeenLastCalledWith(
        { before: null, limit: 25, status: 'disproved' },
        expect.anything(),
      ),
    );
    expect(screen.getByRole('link', { name: 'Submit a conjecture' })).toHaveAttribute(
      'href',
      '/submit',
    );
  });

  it('uses one leaderboard with period and entrant filters and shows agent ownership', async () => {
    const user = userEvent.setup();
    const leaderboard = vi.fn(async () => [row]);
    renderWithApp(<LeaderboardPage />, fakeApi({ leaderboard }));
    expect(await screen.findByRole('link', { name: agent.name })).toHaveAttribute(
      'href',
      '/entrants/agent%3Aproof',
    );
    expect(screen.getByText(`by ${author.display_name}`)).toBeInTheDocument();
    expect(screen.getAllByRole('table')).toHaveLength(1);
    await user.click(screen.getByRole('button', { name: 'Agents' }));
    await user.click(screen.getByRole('button', { name: 'This month' }));
    await waitFor(() =>
      expect(leaderboard).toHaveBeenLastCalledWith('month', 'agents', expect.anything()),
    );
    expect(screen.getByRole('button', { name: 'Agents' })).toHaveAttribute('aria-pressed', 'true');
  });

  it('links a profile’s verified answer to its conjecture and exposes the receipt', async () => {
    renderWithApp(
      <EntrantPage id={agent.id} />,
      fakeApi({
        entrantProfile: vi.fn(async () => ({
          entrant: agent,
          others: 1,
          total: 1,
          errors_found: 0,
          formalized: [],
          solutions: [
            {
              record: detail.summary.record,
              claim: 'C2',
              title: detail.summary.title,
              attempt: 'first',
              receipt,
            },
          ],
        })),
      }),
    );
    expect(await screen.findByRole('heading', { name: agent.name })).toBeInTheDocument();
    expect(screen.getByRole('link', { name: detail.summary.title })).toHaveAttribute(
      'href',
      '/conjectures/WP-2026-0001/C2',
    );
    expect(screen.getByText('Verification receipt')).toBeInTheDocument();
  });

  it('puts research before conjectures and gives readers clear browse and submit actions', async () => {
    const leaderboard = vi.fn(async () => []);
    renderWithApp(
      <HomePage />,
      fakeApi({
        listConjectures: vi.fn(async () => ({ items: [detail.summary], next_before: null })),
        listPapers: vi.fn(async () => ({
          items: [{ ...paperSummary, new_results: 1 }],
          next_before: null,
        })),
        leaderboard,
      }),
    );
    expect(await screen.findByRole('link', { name: paperSummary.title })).toBeInTheDocument();
    expect(await screen.findByRole('link', { name: 'An open bound' })).toBeInTheDocument();
    const intro = screen.getByRole('region', { name: 'Where mathematics takes form.' });
    expect(within(intro).getByRole('link', { name: 'Browse papers' })).toHaveAttribute(
      'href',
      '/papers',
    );
    expect(
      within(intro).getByRole('link', { name: 'Submit a paper or conjecture' }),
    ).toHaveAttribute('href', '/submit');
    expect(within(intro).getByRole('link', { name: 'Contribute a formalization' })).toHaveAttribute(
      'href',
      '/submit?type=lean',
    );
    const research = screen.getByRole('heading', { name: 'Latest research' });
    const conjectures = screen.getByRole('heading', { name: 'Open conjectures' });
    expect(
      research.compareDocumentPosition(conjectures) & Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
    expect(screen.queryByRole('heading', { name: 'Leaderboard' })).not.toBeInTheDocument();
    expect(leaderboard).not.toHaveBeenCalled();
    const css = readFileSync('src/styles/app.css', 'utf8');
    expect(css).toMatch(/\.page pre\s*\{[^}]*max-width:\s*100%;[^}]*overflow-wrap:\s*anywhere/s);
    expect(css).toMatch(/input\[type='file'\]\s*\{\s*max-width:\s*100%/);
  });

  it('keeps research and submission in the main nav and community links in the footer', async () => {
    window.history.replaceState({}, '', '/');
    renderWithApp(<App />, fakeApi({}, author));
    const nav = screen.getByRole('navigation', { name: 'Main' });
    await within(nav).findByRole('link', { name: 'My work' });
    expect(
      within(nav)
        .getAllByRole('link')
        .map((a) => a.textContent),
    ).toEqual(['Papers', 'Conjectures', 'Formalizations', 'Submit', 'My work']);
    expect(within(nav).queryByRole('link', { name: 'Policy' })).not.toBeInTheDocument();
    for (const name of ['Leaderboard', 'Contribute']) {
      expect(within(nav).queryByRole('link', { name })).not.toBeInTheDocument();
      expect(
        within(screen.getByRole('navigation', { name: 'Footer' })).getByRole('link', { name }),
      ).toBeInTheDocument();
    }
    expect(
      within(screen.getByRole('contentinfo')).getByRole('link', { name: 'Policy' }),
    ).toHaveAttribute('href', '/policy');
  });
});
