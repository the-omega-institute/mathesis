import { screen, waitFor, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { it, expect, vi } from 'vitest';
import { fakeApi, renderWithApp } from '../test/render';
import { author, paperSummary } from '../test/fixtures';
import { SubmitPage } from '../pages/Submit';
import type { NewLeanDeposit } from '../api/types';

it('deposits source and original attribution without presenting it as verified', async () => {
  const submitFormalization = vi.fn(async (body: NewLeanDeposit) => ({
    ...body,
    modules: undefined,
    id: 'lean-test',
    owner: author.id,
    source: { id: 'blob', bytes: 12, sha256: 'digest' },
    submitted_at: '2026-10-13T00:00:00Z',
    attempts: [],
    revision: 0,
  }));
  const api = fakeApi({ submitFormalization }, author);
  renderWithApp(<SubmitPage />, api);
  const user = userEvent.setup();
  await screen.findByLabelText('Choose manuscript files');
  await user.click(screen.getByRole('button', { name: 'Community formalization' }));
  await user.click(
    screen.getByText('Deposit an existing formalization without a linked statement'),
  );
  await user.type(screen.getByLabelText('Title', { exact: true }), 'A formal proof');
  await user.clear(screen.getByLabelText('Original authors 1'));
  await user.type(screen.getByLabelText('Original authors 1'), 'Original author');
  await user.type(screen.getByLabelText('Mathematical statement'), 'True');
  await user.type(
    screen.getByLabelText('Original research or source URL'),
    'https://example.org/proof.lean',
  );
  await user.type(screen.getByLabelText('Lean / Mathlib version pin'), 'test-toolchain');
  await user.type(screen.getByLabelText('Source license'), 'Apache-2.0');
  const file = new File(['import Target'], 'Solution.lean', { type: 'text/plain' });
  Object.defineProperty(file, 'text', { value: async () => 'import Target' });
  await user.upload(screen.getByLabelText('Solution.lean / standalone source (up to 1 MB)'), file);
  // jsdom does not validate userEvent's synthetic FileList correctly; native submission is covered by browser QA.
  fireEvent.submit(screen.getByRole('form', { name: 'Submit Lean formalization' }));
  await waitFor(() =>
    expect(submitFormalization).toHaveBeenCalledWith(
      expect.objectContaining({
        authors: ['Original author'],
        solution: 'import Target',
        toolchain: 'test-toolchain',
      }),
    ),
  );
  expect(await screen.findByRole('status')).toHaveTextContent('Verification has not run yet');
  expect(api.verifyFormalization).not.toHaveBeenCalled();
});

it('prefills the confirmed statement and binds a multi-file verification request to its digest', async () => {
  const detail = {
    record: 'WP-2026-0001',
    title: 'Original paper',
    claim: {
      id: 'C1',
      label: 'Theorem 1',
      kind: 'theorem' as const,
      role: 'main' as const,
      statement: 'True',
      depends_on: [],
      lean: null,
    },
    target: {
      claim: 'C1',
      lean: 'import Mathlib',
      digest: 'confirmed-digest',
      toolchain: 'pinned-toolchain',
    },
    verified_attempts: [],
    macros: {},
  };
  const submitFormalization = vi.fn(async (body: NewLeanDeposit) => ({
    ...body,
    modules: {},
    id: 'lean-project',
    owner: author.id,
    source: { id: 'blob', bytes: 12, sha256: 'digest' },
    submitted_at: '2026-10-13T00:00:00Z',
    attempts: [],
    revision: 0,
  }));
  const api = fakeApi(
    {
      getStatement: vi.fn(async () => detail),
      getPaper: vi.fn(async () => ({
        summary: { ...paperSummary, authors: [{ name: 'Original researcher' }] },
        claims: [],
      })),
      submitFormalization,
      verifyFormalization: vi.fn(async () => ({
        id: 'attempt',
        record: detail.record,
        claim: 'C1',
        entrant: {
          id: author.id,
          name: author.display_name,
          kind: 'person' as const,
          retired: false,
          revision: 0,
        },
        state: 'queued' as const,
        receipt: null,
        solution: '',
      })),
    },
    author,
  );
  renderWithApp(
    <SubmitPage initialIntake="lean" targetRecord={detail.record} targetClaim="C1" />,
    api,
  );
  expect(await screen.findByLabelText('Lean / Mathlib version pin')).toHaveValue(
    'pinned-toolchain',
  );
  expect(screen.getByLabelText('Original authors 1')).toHaveValue('Original researcher');
  const user = userEvent.setup();
  await user.type(screen.getByLabelText('Source license'), 'MIT');
  const main = new File(['import Target\nimport Helpers'], 'Solution.lean', { type: 'text/plain' });
  Object.defineProperty(main, 'text', { value: async () => 'import Target\nimport Helpers' });
  const helper = new File(['theorem helper : True := True.intro'], 'Helpers.lean', {
    type: 'text/plain',
  });
  Object.defineProperty(helper, 'text', {
    value: async () => 'theorem helper : True := True.intro',
  });
  await user.upload(screen.getByLabelText('Solution.lean / standalone source (up to 1 MB)'), main);
  await user.upload(screen.getByLabelText('Supporting Lean modules (optional)'), helper);
  fireEvent.submit(screen.getByRole('form', { name: 'Submit Lean formalization' }));
  await waitFor(() =>
    expect(api.verifyFormalization).toHaveBeenCalledWith(
      'lean-project',
      detail.record,
      'C1',
      'confirmed-digest',
    ),
  );
  expect(submitFormalization).toHaveBeenCalledWith(
    expect.objectContaining({
      modules: { Helpers: 'theorem helper : True := True.intro' },
      authors: ['Original researcher'],
      formalizers: [author.display_name],
    }),
  );
  expect(await screen.findByRole('status')).toHaveTextContent('Verification queued');
});

it('deposits a linked Lean source with its revision and separate author credits', async () => {
  const revision = 'a'.repeat(40);
  const url = `https://github.com/example/research/blob/${revision}/Proof.lean`;
  const source = 'import Mathlib\ntheorem example_result : True := True.intro';
  const fetchSource = vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response(source));
  const submitFormalization = vi.fn(async (body: NewLeanDeposit) => ({
    ...body,
    modules: {},
    id: 'linked-source',
    owner: author.id,
    source: { id: 'blob', bytes: source.length, sha256: 'digest' },
    submitted_at: '2026-11-02T00:00:00Z',
    attempts: [],
    revision: 0,
  }));
  const api = fakeApi({ submitFormalization }, author);
  try {
    renderWithApp(<SubmitPage initialIntake="lean" />, api);
    const user = userEvent.setup();
    await user.click(
      await screen.findByText('Deposit an existing formalization without a linked statement'),
    );
    await user.type(screen.getByLabelText('Title', { exact: true }), 'A reusable lemma');
    await user.clear(screen.getByLabelText('Original authors 1'));
    await user.type(screen.getByLabelText('Original authors 1'), 'Original mathematician');
    await user.type(screen.getByLabelText('Mathematical statement'), 'True');
    await user.type(screen.getByLabelText('Lean / Mathlib version pin'), 'test-toolchain');
    await user.type(screen.getByLabelText('Source license'), 'Apache-2.0');
    await user.type(screen.getByLabelText('Link to Lean code'), url);
    await user.click(screen.getByRole('button', { name: 'Import Lean source' }));
    expect(await screen.findByRole('status')).toHaveTextContent('Imported Proof.lean');
    const adapterText =
      'import Proof\nimport Target\ntheorem wishpool_solution : wishpool_target_prop := by trivial';
    const adapter = new File([adapterText], 'Solution.lean');
    Object.defineProperty(adapter, 'text', { value: async () => adapterText });
    await user.upload(
      screen.getByLabelText('Solution.lean / standalone source (up to 1 MB)'),
      adapter,
    );
    fireEvent.submit(screen.getByRole('form', { name: 'Submit Lean formalization' }));
    await waitFor(() =>
      expect(submitFormalization).toHaveBeenCalledWith(
        expect.objectContaining({
          solution: adapterText,
          modules: { Proof: source },
          authors: ['Original mathematician'],
          formalizers: [author.display_name],
          source_revision: revision,
          source_reference: { kind: 'url', locator: url },
        }),
      ),
    );
    expect(api.verifyFormalization).not.toHaveBeenCalled();
    expect(await screen.findByText(/Source received:/)).toHaveTextContent(
      'Verification has not run yet',
    );
  } finally {
    fetchSource.mockRestore();
  }
});

it('opens community proof contribution for a collected result and preserves its exact original authors', async () => {
  const { dependencyPaper } = await import('../test/dependencyFixtures');
  const api = fakeApi(
    {
      getStatement: vi.fn(async () => ({
        record: 'gap-note',
        title: dependencyPaper.title,
        publication: { version: 2, claims_revision: 0 },
        claim: {
          id: 's2',
          label: 'Theorem 2',
          kind: 'theorem' as const,
          role: 'main' as const,
          statement: 'A precise bound.',
          depends_on: [],
        },
        target: {
          claim: 's2',
          lean: 'import Mathlib',
          digest: 'exact-target',
          toolchain: 'pinned-toolchain',
        },
        verified_attempts: [],
        macros: {},
      })),
      getCollectedPaper: vi.fn(async () => dependencyPaper),
    },
    author,
  );
  renderWithApp(<SubmitPage initialIntake="lean" targetRecord="gap-note" targetClaim="s2" />, api);
  expect(await screen.findByLabelText('Original authors 1')).toHaveValue('Original Author');
  expect(screen.getByLabelText('Mathematical statement')).toHaveValue('A precise bound.');
  expect(api.getCollectedPaper).toHaveBeenCalledWith('gap-note', '2', expect.any(Object));
  expect(api.getPaper).not.toHaveBeenCalled();
});
