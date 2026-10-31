import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { CheckedContribution, FormalizationWork, ResultUse } from '../api/types';
import { author } from '../test/fixtures';
import { fakeApi, renderWithApp } from '../test/render';
import { ProofWork } from './ProofWork';
import { CheckedResultUses } from './ResultConnections';

const work: FormalizationWork = {
  target_digest: 'private-target-digest',
  total_generations: 3,
  used_generations: 2,
  expires_at: '2099-11-02T00:00:00Z',
  paused: true,
  state: 'paused',
  can_control: true,
  managed: true,
};
const contribution: CheckedContribution = {
  origin: {
    record: 'WP-2026-0001',
    claim: 'C1',
    version: 1,
    claims_revision: 2,
    attempt: 'helper-attempt',
    target_digest: 'pin',
    solution_digest: 'source-hash',
    declaration: 'Research.helper',
  },
  intermediate: {
    name: 'Research.helper',
    title: 'The auxiliary bound',
    proposition: 'True ∧ True',
    license: 'CC-BY-4.0',
  },
  authors: [{ name: 'Original Author' }],
  contributor: {
    id: 'original-solver',
    name: 'Original Formalizer',
    kind: 'person',
    retired: false,
    revision: 0,
  },
  attribution: null,
  source_sha256: 'source-sha',
  receipt: {
    verdict: 'proved',
    reason: 'Independent verifier checked the proposition',
    target_digest: 'auxiliary-hash',
    solution_digest: 'source-hash',
    toolchain: 'internal-toolchain-pin',
    axioms: [],
    checked_at: '2026-11-01T00:00:00Z',
    duration: 1,
  },
  verification_request: {
    target: 'target',
    target_digest: 'auxiliary-hash',
    toolchain: 'internal-toolchain-pin',
    solution: 'adapter',
    modules: { 'Research.Helpers': 'source' },
  },
};

describe('current proof work', () => {
  it('retains total consumption and expiry when the author resumes', async () => {
    const user = userEvent.setup();
    const control = vi.fn(async () => ({ ...work, paused: false, state: 'active' as const }));
    renderWithApp(
      <ProofWork record="WP-2026-0001" claim="C1" />,
      fakeApi(
        {
          formalizationWork: vi.fn(async () => work),
          controlFormalization: control,
          checkedContributions: vi.fn(async () => [contribution]),
        },
        author,
      ),
    );
    expect(await screen.findByText(/2 of 3 authorized generations/)).toBeVisible();
    expect(screen.getByText('The auxiliary bound')).toBeVisible();
    expect(screen.getByText(/Original Author/)).toBeVisible();
    expect(screen.getByRole('link', { name: 'Original Formalizer' })).toBeVisible();
    expect(screen.queryByText('private-target-digest')).toBeNull();
    expect(screen.queryByText('internal-toolchain-pin')).toBeNull();
    await user.click(screen.getByText('Automatic proof work allowance'));
    await user.click(screen.getByRole('button', { name: 'Resume saved work' }));
    await waitFor(() =>
      expect(control).toHaveBeenCalledWith('WP-2026-0001', 'C1', {
        target_digest: work.target_digest,
        total_generations: 3,
        expires_at: work.expires_at,
        paused: false,
        change_allowance: false,
      }),
    );
    expect(screen.getByText(/2 of 3 authorized generations/)).toBeVisible();
  });

  it('requires an explicit allowance save to change the authorized total', async () => {
    const user = userEvent.setup();
    const control = vi.fn(async () => work);
    renderWithApp(
      <ProofWork record="WP-2026-0001" claim="C1" />,
      fakeApi(
        {
          formalizationWork: vi.fn(async () => ({ ...work, state: 'not_authorized' as const })),
          controlFormalization: control,
          checkedContributions: vi.fn(async () => []),
        },
        author,
      ),
    );
    await screen.findByText(/2 of 3 authorized generations/);
    expect(screen.getByText(/The confirmed statement changed/)).toBeVisible();
    await user.click(screen.getByText('Automatic proof work allowance'));
    const total = screen.getByRole('spinbutton', { name: 'Total generations' });
    await user.clear(total);
    await user.type(total, '5');
    await user.click(screen.getByRole('button', { name: 'Save allowance' }));
    await waitFor(() =>
      expect(control).toHaveBeenCalledWith(
        'WP-2026-0001',
        'C1',
        expect.objectContaining({
          total_generations: 5,
          paused: true,
          change_allowance: true,
        }),
      ),
    );
  });

  it('shows checked intermediate scope while the final goal is unfinished, without granting reader controls', async () => {
    renderWithApp(
      <ProofWork record="WP-2026-0001" claim="C1" />,
      fakeApi({
        formalizationWork: vi.fn(async () => ({
          ...work,
          can_control: false,
          state: 'allowance_used' as const,
        })),
        checkedContributions: vi.fn(async () => [contribution]),
      }),
    );
    expect(await screen.findByText(/authorized allowance has been used/)).toBeVisible();
    expect(screen.getByText(/final goal may remain unfinished/)).toBeVisible();
    expect(screen.queryByText('Automatic proof work allowance')).toBeNull();
    expect(
      screen.getByRole('link', { name: 'Download proposition, sources and verification details' }),
    ).toHaveAttribute('href', '/api/v1/contributions/helper-attempt/bundle');
  });

  it('withholds old content when a fresh public read cannot resolve the goal', async () => {
    renderWithApp(<ProofWork record="WP-2026-0001" claim="C1" />, fakeApi());
    await waitFor(() => expect(screen.queryByText('Current proof work')).toBeNull());
    expect(screen.queryByText('Original Formalizer')).toBeNull();
  });

  it('finds another public result and submits selected original contributions under the current target', async () => {
    const user = userEvent.setup();
    const checked = vi.fn(async (record: string) =>
      record === 'WP-2026-0001' ? [contribution] : [],
    );
    const submit = vi.fn(async () => ({
      id: 'queued-proof',
      record: 'WP-2026-0002',
      claim: 'C2',
      entrant: contribution.contributor,
      state: 'queued' as const,
      receipt: null,
      solution: '',
    }));
    renderWithApp(
      <ProofWork record="WP-2026-0002" claim="C2" />,
      fakeApi(
        {
          formalizationWork: vi.fn(async () => ({ ...work, can_control: false })),
          checkedContributions: checked,
          submitReusingAttempt: submit,
        },
        author,
      ),
    );
    await screen.findByText('Checked propositions');
    await user.click(screen.getByText('Use checked propositions in a proof'));
    await user.type(
      screen.getByRole('textbox', { name: 'Another Mathesis result link' }),
      `${window.location.origin}/papers/WP-2026-0001/results/C1`,
    );
    await user.click(screen.getByRole('button', { name: 'Find checked propositions' }));
    await user.click(await screen.findByRole('checkbox', { name: /The auxiliary bound/ }));
    const proof =
      'import Target\nimport Research.Helpers\ntheorem wishpool_solution : wishpool_target_prop := Research.helper';
    await user.type(screen.getByRole('textbox', { name: 'Complete Lean proof' }), proof);
    await user.click(screen.getByRole('button', { name: 'Submit proof with checked sources' }));
    await waitFor(() =>
      expect(submit).toHaveBeenCalledWith('WP-2026-0002', 'C2', {
        expected_target: work.target_digest,
        solution: proof,
        contributions: [contribution.origin.attempt],
      }),
    );
    expect(await screen.findByRole('status')).toHaveTextContent(
      'Submitted for independent checking with exact original sources and attribution',
    );
  });
});

describe('checked result relationships', () => {
  const relation: ResultUse = {
    result: {
      record: 'WP-2026-0002',
      claim: 'C2',
      version: 3,
      claims_revision: 4,
      title: 'A later theorem',
      label: 'Theorem 2',
      authors: [{ name: 'Later Author' }],
    },
    used_result: {
      record: 'WP-2026-0001',
      claim: 'C1',
      version: 1,
      claims_revision: 2,
      title: 'Earlier work',
      label: 'Lemma 1 [lem:bound]',
      authors: [{ name: 'Original Author' }],
    },
    contribution: contribution.intermediate,
    contributor: contribution.contributor,
    attribution: null,
    attempt: 'checked-main',
    source_attempt: 'helper-attempt',
    basis: 'checked_proof_term',
    joint_context: [contribution.origin, { ...contribution.origin, attempt: 'second-helper' }],
  };
  it('shows exact versions, original credit and the joint-context limit in the forward direction', async () => {
    renderWithApp(
      <CheckedResultUses record="WP-2026-0002" claim="C2" />,
      fakeApi({ resultUses: vi.fn(async () => [relation]) }),
    );
    expect(
      await screen.findByRole('heading', { name: 'Uses propositions from these results' }),
    ).toBeVisible();
    expect(screen.getByRole('link', { name: 'Lemma 1 — Earlier work' })).toHaveAttribute(
      'href',
      '/papers/WP-2026-0001/results/C1',
    );
    expect(
      screen.getByText(/2 contributed propositions; individual sufficiency is not asserted/),
    ).toBeVisible();
    expect(screen.getByText(/source result itself may remain unfinished/)).toBeVisible();
    expect(screen.getByRole('link', { name: 'Original Formalizer' })).toBeVisible();
  });
  it('offers the inverse link without turning bibliography citations into checked use', async () => {
    renderWithApp(
      <CheckedResultUses record="WP-2026-0001" claim="C1" />,
      fakeApi({ resultUses: vi.fn(async () => [relation]) }),
    );
    expect(
      await screen.findByRole('heading', { name: 'Propositions used in these results' }),
    ).toBeVisible();
    expect(screen.getByRole('link', { name: 'Theorem 2 — A later theorem' })).toHaveAttribute(
      'href',
      '/papers/WP-2026-0002/results/C2',
    );
    expect(
      screen.getByText(/Citations and unused imports do not establish checked use/),
    ).toBeVisible();
  });
});
