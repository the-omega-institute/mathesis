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
  await screen.findByLabelText('LaTeX source (required)');
  await user.click(screen.getByRole('button', { name: 'Lean formalization' }));
  await user.type(screen.getByLabelText('Title', { exact: true }), 'A formal proof');
  await user.clear(screen.getByLabelText('Original authors (one per line)'));
  await user.type(screen.getByLabelText('Original authors (one per line)'), 'Original author');
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
  expect(screen.getByLabelText('Original authors (one per line)')).toHaveValue(
    'Original researcher',
  );
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
