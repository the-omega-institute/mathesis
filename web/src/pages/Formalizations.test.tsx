import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { it, expect, vi } from 'vitest';
import { fakeApi, renderWithApp } from '../test/render';
import { FormalizationsPage } from './Formalizations';
import type { FormalizationOpportunity } from '../api/types';
import { dependencyPaper } from '../test/dependencyFixtures';

const opportunity: FormalizationOpportunity = {
  record: 'WP-2026-0001',
  claim: 'C1',
  title: 'Prime gaps',
  label: 'Theorem 1',
  statement: 'A checked mathematical statement',
  authors: [{ name: 'Research author' }],
  kind: 'paper',
  status: 'open',
  target_digest: 'exact-target',
  toolchain: 'pinned',
  verified_count: 0,
  contributor: null,
  attribution: null,
  receipt: null,
};
it('links to an attributed contribution and asks the API to filter the full directory', async () => {
  const list = vi.fn(async () => ({ items: [opportunity], next_before: null }));
  renderWithApp(<FormalizationsPage view="list" />, fakeApi({ formalizationOpportunities: list }));
  expect(await screen.findByRole('link', { name: 'Contribute a proof' })).toHaveAttribute(
    'href',
    '/submit?type=lean&record=WP-2026-0001&claim=C1',
  );
  expect(screen.getByText('Target ready')).toBeInTheDocument();
  expect(list).toHaveBeenCalledWith(
    expect.objectContaining({ status: 'unverified' }),
    expect.anything(),
  );
  const user = userEvent.setup();
  await user.click(screen.getByRole('button', { name: 'Verified proofs' }));
  await user.selectOptions(screen.getByLabelText('Statement type'), 'paper');
  await user.type(screen.getByLabelText('Find a statement'), 'prime gaps');
  await user.click(screen.getByRole('button', { name: 'Search' }));
  await waitFor(() =>
    expect(list).toHaveBeenLastCalledWith(
      expect.objectContaining({ status: 'verified', kind: 'paper', q: 'prime gaps' }),
      expect.anything(),
    ),
  );
});
it('offers source intake when no results are awaiting formalization', async () => {
  renderWithApp(<FormalizationsPage view="list" />, fakeApi());
  expect(await screen.findByText('No mathematical results listed yet')).toBeInTheDocument();
  expect(screen.getByRole('link', { name: 'Submit existing Lean work →' })).toHaveAttribute(
    'href',
    '/submit?type=lean',
  );
});

it('shows pending mathematics and authors without offering an unavailable proof target', async () => {
  const pending: FormalizationOpportunity = {
    ...opportunity,
    record: 'prime-note',
    version: 2,
    status: 'preparing',
    statement: 'For $n \\in \\NN$, we have $n=n$.',
    macros: { '\\NN': '\\mathbb{N}' },
    target_digest: null,
    toolchain: null,
  };
  renderWithApp(
    <FormalizationsPage view="list" />,
    fakeApi({ formalizationOpportunities: async () => ({ items: [pending], next_before: null }) }),
  );
  expect(await screen.findByText('Awaiting formalization')).toBeInTheDocument();
  expect(screen.getByText('Research author')).toBeInTheDocument();
  expect(screen.getByRole('link', { name: 'Prime gaps' })).toHaveAttribute(
    'href',
    '/papers/prime-note?version=2',
  );
  expect(screen.getByRole('link', { name: 'Read result →' })).toHaveAttribute(
    'href',
    '/papers/prime-note/results/C1?version=2',
  );
  expect(document.querySelector('.katex-error')).not.toBeInTheDocument();
  expect(document.querySelector('.katex')).toBeInTheDocument();
  expect(screen.queryByRole('link', { name: 'Contribute a proof' })).not.toBeInTheDocument();
  expect(screen.queryByText('Lean ✓')).not.toBeInTheDocument();
});

it('displays the existing pinned formalization without asserting that every extracted statement has a new checked proof', async () => {
  const item: FormalizationOpportunity = {
    ...opportunity,
    record: dependencyPaper.id,
    version: 2,
    status: 'preparing',
    target_digest: null,
    toolchain: null,
  };
  const api = fakeApi({
    formalizationOpportunities: async () => ({
      items: [item, { ...item, claim: 'C2' }],
      next_before: null,
    }),
    getCollectedPaper: vi.fn(async () => dependencyPaper),
  });
  renderWithApp(<FormalizationsPage view="list" />, api);
  expect(await screen.findAllByText('Lean source available')).toHaveLength(2);
  expect(screen.getAllByRole('link', { name: 'Read the Lean source' })[0]).toHaveAttribute(
    'href',
    dependencyPaper.lean!.url,
  );
  expect(screen.queryByText('Awaiting formalization')).toBeNull();
  expect(screen.queryByText('Lean ✓')).toBeNull();
  expect(api.getCollectedPaper).toHaveBeenCalledTimes(1);
});

it("never borrows a newer version's Lean source for an older result", async () => {
  renderWithApp(
    <FormalizationsPage view="list" />,
    fakeApi({
      formalizationOpportunities: async () => ({
        items: [
          {
            ...opportunity,
            record: dependencyPaper.id,
            version: 1,
            status: 'preparing',
            target_digest: null,
          },
        ],
        next_before: null,
      }),
      getCollectedPaper: async () => dependencyPaper,
    }),
  );
  expect(await screen.findByText('Awaiting formalization')).toBeVisible();
  expect(screen.queryByRole('link', { name: 'Read the Lean source' })).toBeNull();
});

it('shows whether a verified proof rests on library declarations or is new Lean, and filters by it', async () => {
  const revision = 'b'.repeat(40);
  const receipt = {
    verdict: 'proved' as const,
    reason: 'Kernel checked.',
    target_digest: 'exact-target',
    solution_digest: 'solution',
    toolchain: 'pinned',
    axioms: [],
    checked_at: '2026-11-11T00:00:00Z',
    duration: 1,
  };
  const fromLibrary: FormalizationOpportunity = {
    ...opportunity,
    status: 'verified',
    verified_count: 1,
    receipt,
    proof_basis: {
      kind: 'library_declarations',
      libraries: [{ name: 'trureturing', revision, url: 'https://github.com/example/library' }],
      declarations: [
        {
          library: 'trureturing',
          module: 'D5.S3.Result',
          name: 'D5.S3.Result.result',
          source: `https://github.com/example/library/blob/${revision}/D5/S3/Result.lean`,
        },
      ],
      total: 1,
      complete: true,
    },
  };
  const newLean: FormalizationOpportunity = {
    ...opportunity,
    claim: 'C2',
    label: 'Lemma 2',
    status: 'verified',
    verified_count: 1,
    receipt,
    proof_basis: { kind: 'new_lean' },
  };
  const list = vi.fn(async (query?: { status?: string; basis?: string }) => ({
    items:
      query?.status !== 'verified'
        ? [opportunity]
        : [fromLibrary, newLean].filter(
            (item) => !query.basis || item.proof_basis?.kind === query.basis,
          ),
    next_before: null,
  }));
  renderWithApp(<FormalizationsPage view="list" />, fakeApi({ formalizationOpportunities: list }));
  const user = userEvent.setup();
  // The basis filter belongs to verified proofs only.
  expect(await screen.findByText('Target ready')).toBeInTheDocument();
  expect(screen.queryByRole('group', { name: 'Proof basis' })).toBeNull();
  expect(screen.queryByText('New Lean proof')).toBeNull();
  await user.click(screen.getByRole('button', { name: 'Verified proofs' }));
  expect(await screen.findByText('Proved from library declarations')).toBeVisible();
  expect(screen.getByText('New Lean proof')).toBeVisible();
  expect(screen.getByText('D5.S3.Result.result')).toBeInTheDocument();
  expect(
    screen.getByRole('link', { name: 'Library source of D5.S3.Result.result' }),
  ).toHaveAttribute(
    'href',
    `https://github.com/example/library/blob/${revision}/D5/S3/Result.lean`,
  );
  expect(list).toHaveBeenLastCalledWith(
    expect.objectContaining({ status: 'verified', basis: undefined }),
    expect.anything(),
  );
  const basis = screen.getByRole('group', { name: 'Proof basis' });
  await user.click(within(basis).getByRole('button', { name: 'From library declarations' }));
  await waitFor(() =>
    expect(list).toHaveBeenLastCalledWith(
      expect.objectContaining({ status: 'verified', basis: 'library_declarations' }),
      expect.anything(),
    ),
  );
  await waitFor(() => expect(screen.queryByText('New Lean proof')).toBeNull());
  expect(screen.getByText('Proved from library declarations')).toBeVisible();
  await user.click(within(basis).getByRole('button', { name: 'New Lean proofs' }));
  expect(await screen.findByText('New Lean proof')).toBeVisible();
  expect(screen.queryByText('Proved from library declarations')).toBeNull();
  // Leaving the verified view drops the basis filter.
  await user.click(screen.getByRole('button', { name: 'Explore results' }));
  await waitFor(() =>
    expect(list).toHaveBeenLastCalledWith(
      expect.objectContaining({ status: 'unverified', basis: undefined }),
      expect.anything(),
    ),
  );
  expect(screen.queryByRole('group', { name: 'Proof basis' })).toBeNull();
  expect(document.body.textContent).not.toMatch(/equivalent/i);
});
