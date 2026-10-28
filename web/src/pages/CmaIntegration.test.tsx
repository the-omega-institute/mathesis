import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { ApiError } from '../api/client';
import type { Person } from '../api/types';
import { CMA_COPY } from '../components/MyCma';
import { author, draft, editor, submission } from '../test/fixtures';
import { fakeApi, renderWithApp } from '../test/render';
import { ContributePage } from './Contribute';
import { SubmissionDetailPage } from './SubmissionDetail';
import { SubmitPage } from './Submit';

const privateFields = {
  workspace: 'PRIVATE_WORKSPACE_SENTINEL',
  owner: 'PRIVATE_OWNER_SENTINEL',
  binding_generation: 'PRIVATE_GENERATION_SENTINEL',
  progress: 'PRIVATE_PROGRESS_SENTINEL',
  access_token: 'PRIVATE_ACCESS_SENTINEL',
  refresh_token: 'PRIVATE_REFRESH_SENTINEL',
  sealed: 'PRIVATE_SEALED_SENTINEL',
};
const attempts = [
  {
    ...privateFields,
    kind: 'audit' as const,
    round: 1,
    step: 'r1:audit',
    attempt: 0,
    payer: 'author' as const,
    reason: 'author_connected',
    status: 'running' as const,
    admitted_at: '2026-10-11T00:00:00Z',
    finished_at: null,
  },
];

describe('Optional own agents in pages', () => {
  it.each(['submitter', 'coauthor', 'staff', 'public'] as const)(
    'shows pre-letter history to %s with submitter-only connection controls and safe fields',
    async (role) => {
      const coauthor = { ...author, id: 'linked-coauthor', display_name: 'Linked author' };
      const readers: Record<string, Person | undefined> = {
        submitter: author,
        coauthor,
        staff: editor,
        public: undefined,
      };
      const paper = {
        ...submission,
        authors: [...submission.authors, { name: coauthor.display_name, person: coauthor.id }],
      };
      const getCma = vi.fn(async () => ({
        enabled: true,
        setup_url: 'https://cma.example/auth/login?return_to=/',
        binding: {
          person: author.id,
          subject: author.id,
          revision: 0,
          generation: 0,
          status: 'disconnected' as const,
          reason: 'not_connected',
          workspace_id: null,
          sandbox_id: null,
          sandbox_profile: null,
          checked_at: null,
          created_at: '2026-10-11T00:00:00Z',
          updated_at: '2026-10-11T00:00:00Z',
          ...privateFields,
        },
      }));
      const referee = vi.fn(async () => ({
        id: paper.id,
        revision: 1,
        rounds: [],
        letters: [],
        cma_attempts: attempts,
        ...privateFields,
      }));
      renderWithApp(
        <SubmissionDetailPage id={paper.id} />,
        fakeApi({ getSubmission: vi.fn(async () => paper), referee, getCma }, readers[role]),
      );
      await screen.findByRole('heading', { name: paper.title });
      if (role === 'public') {
        expect(referee).not.toHaveBeenCalled();
        expect(screen.queryByRole('region', { name: 'Execution records' })).toBeNull();
      } else {
        const advanced = screen
          .getByText('Advanced: compute settings and history')
          .closest('details');
        expect(advanced).not.toHaveAttribute('open');
        if (role !== 'staff')
          await userEvent.click(screen.getByText('Details: evidence, history and settings'));
        await userEvent.click(screen.getByText('Advanced: compute settings and history'));
        expect(await screen.findByRole('region', { name: 'Execution records' })).toHaveTextContent(
          'Source check · Your agents · In progress',
        );
        expect(referee).toHaveBeenCalled();
      }
      if (role === 'submitter') {
        expect(await screen.findByRole('link', { name: 'Connect' })).toBeInTheDocument();
        expect(getCma).toHaveBeenCalled();
      } else {
        expect(screen.queryByRole('link', { name: 'Connect' })).toBeNull();
        expect(getCma).not.toHaveBeenCalled();
      }
      for (const sentinel of Object.values(privateFields)) {
        expect(document.body.innerHTML).not.toContain(sentinel);
      }
    },
  );

  it('keeps optional compute settings in the advanced section', async () => {
    renderWithApp(<ContributePage />, fakeApi({}, author));
    const advanced = screen.getByText('Advanced: use your own compute account').closest('details');
    expect(advanced).toHaveAttribute('id', 'own-agents');
    expect(advanced).not.toHaveAttribute('open');
    await userEvent.click(screen.getByText('Advanced: use your own compute account'));
    expect(
      await screen.findByRole('region', { name: 'Use your own agents (optional)' }),
    ).toBeInTheDocument();
  });

  it('offers research and Lean intake without requiring a compute account', async () => {
    const api = fakeApi({}, author);
    renderWithApp(<SubmitPage />, api);
    await screen.findByLabelText('Title');
    expect(screen.getByRole('button', { name: 'Community formalization' })).toBeInTheDocument();
    expect(screen.queryByText(CMA_COPY)).toBeNull();
    expect(api.getCma).not.toHaveBeenCalled();
  });

  it('uploads without depending on CMA availability', async () => {
    const createSubmission = vi.fn(async () => draft);
    const getCma = vi.fn(async () => {
      throw new ApiError({ status: 503, code: 'unavailable', detail: 'CMA status unavailable' });
    });
    renderWithApp(<SubmitPage />, fakeApi({ getCma, createSubmission }, author));
    const user = userEvent.setup();
    const file = new File(['source'], 'paper.tex', { type: 'application/x-tex' });
    await user.type(await screen.findByLabelText('Title'), 'A result');
    await user.click(screen.getByRole('tab', { name: 'Authors' }));
    await user.type(screen.getByLabelText('Name'), 'Ada Example');
    await user.click(screen.getByRole('tab', { name: 'Files' }));
    await user.upload(screen.getByLabelText('Choose manuscript files'), file);
    await user.click(screen.getByRole('tab', { name: 'Review & submit' }));
    await user.click(screen.getByRole('checkbox', { name: /I agree that the source/ }));
    await user.click(screen.getByRole('button', { name: 'Upload manuscript' }));
    await waitFor(() => expect(createSubmission).toHaveBeenCalledWith(expect.any(Object), file));
    expect(getCma).not.toHaveBeenCalled();
    expect(screen.queryByText('CMA status unavailable')).toBeNull();
  });
});
