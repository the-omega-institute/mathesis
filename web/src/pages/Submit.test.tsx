import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { strFromU8, unzipSync } from 'fflate';
import { expect, it, vi } from 'vitest';
import { author, draft, version } from '../test/fixtures';
import { fakeApi, renderWithApp } from '../test/render';
import type { Submission } from '../api/types';
import { SubmitPage } from './Submit';

const prepared: Submission = {
  ...draft,
  title: 'A bound for finite graphs',
  abstract_text: 'We prove the stated bound.',
  authors: [{ name: 'Ada Example' }],
  intake: { confirmed_request: null },
  versions: [version(1)],
  extracted: draft.extracted.slice(0, 2),
};
async function upload(
  user: ReturnType<typeof userEvent.setup>,
  files: File | File[] = new File(['source'], 'paper.tex'),
) {
  await user.upload(await screen.findByLabelText('Choose manuscript files'), files);
  await user.click(screen.getByRole('button', { name: 'Upload and prepare PDF' }));
}
async function checkPdf(user: ReturnType<typeof userEvent.setup>) {
  const next = await screen.findByRole('button', { name: 'Continue to details' });
  await waitFor(() => expect(next).toBeEnabled());
  await user.click(next);
}
it('starts with upload and keeps metadata unavailable until the PDF is checked', async () => {
  const prepareSubmission = vi.fn(async () => prepared);
  renderWithApp(<SubmitPage />, fakeApi({ prepareSubmission }, author));
  const user = userEvent.setup();
  expect(await screen.findByRole('heading', { name: 'Upload your manuscript' })).toBeVisible();
  expect(screen.queryByLabelText('Title')).toBeNull();
  expect(screen.queryByLabelText('Type of work')).toBeNull();
  expect(screen.queryByRole('checkbox')).toBeNull();
  expect(screen.getByRole('tab', { name: 'Details' })).toBeDisabled();
  expect(screen.queryByRole('button', { name: 'Private review' })).toBeNull();
  await user.click(screen.getByRole('button', { name: 'Upload and prepare PDF' }));
  expect(prepareSubmission).not.toHaveBeenCalled();
  await upload(user);
  expect(prepareSubmission).toHaveBeenCalledWith(
    { kind: 'paper', purpose: 'publication', processing_consent: 'ai-review-request-2026-10-29' },
    expect.any(File),
  );
  expect(screen.getByRole('button', { name: 'Continue to details' })).toBeEnabled();
  expect(screen.getByTitle('Manuscript PDF preview')).toBeVisible();
  await checkPdf(user);
  expect(screen.getByLabelText('Title')).toHaveValue(prepared.title);
  expect(screen.getByLabelText('Abstract')).toHaveValue(prepared.abstract_text);
  await user.clear(screen.getByLabelText('Title'));
  await user.type(screen.getByLabelText('Title'), 'Author correction');
  await user.click(screen.getByRole('tab', { name: 'Authors' }));
  expect(screen.getByLabelText('Name')).toHaveValue('Ada Example');
  await user.click(screen.getByRole('tab', { name: 'Details' }));
  expect(screen.getByLabelText('Title')).toHaveValue('Author correction');
  await user.keyboard('{ArrowRight}');
  expect(screen.getByRole('tab', { name: 'Authors' })).toHaveFocus();
});
it.each(['paper.pdf', 'paper.zip', 'paper.tar.gz'])(
  'prepares %s before requesting any metadata',
  async (name) => {
    const prepareSubmission = vi.fn(async () => prepared);
    renderWithApp(<SubmitPage />, fakeApi({ prepareSubmission }, author));
    const user = userEvent.setup();
    const file = new File(['source'], name);
    await upload(user, file);
    expect(prepareSubmission).toHaveBeenCalledWith(expect.any(Object), file);
  },
);
it('keeps all files when preparing a TeX project', async () => {
  const prepareSubmission = vi.fn(async () => prepared);
  renderWithApp(<SubmitPage />, fakeApi({ prepareSubmission }, author));
  const user = userEvent.setup();
  await upload(user, [
    new File(['main'], 'main.tex'),
    new File(['proof'], 'proof.tex'),
    new File(['refs'], 'refs.bib'),
  ]);
  const file = (prepareSubmission.mock.calls as unknown as [unknown, File][])[0]![1];
  const files = unzipSync(new Uint8Array(await file.arrayBuffer()));
  expect(Object.keys(files)).toEqual(['main.tex', 'proof.tex', 'refs.bib']);
  expect(strFromU8(files['proof.tex']!)).toBe('proof');
});
it('waits for actual compilation and resumes a saved draft', async () => {
  const compiling = { ...prepared, versions: [version(1, { pdf: undefined })] };
  const getSubmission = vi.fn(async () => prepared);
  renderWithApp(<SubmitPage initialSubmission={compiling} />, fakeApi({ getSubmission }, author));
  const user = userEvent.setup();
  await checkPdf(user);
  expect(getSubmission).toHaveBeenCalledWith(prepared.id, expect.any(Object));
  expect(screen.getByLabelText('Title')).toHaveValue(prepared.title);
});
it('keeps failed compilation on the PDF step and offers replacement', async () => {
  const failed = {
    ...prepared,
    versions: [version(1, { pdf: undefined, compile_error: 'Missing figure.pdf' })],
  };
  renderWithApp(<SubmitPage initialSubmission={failed} />, fakeApi({}, author));
  const user = userEvent.setup();
  expect(await screen.findByRole('alert')).toHaveTextContent('could not be compiled');
  expect(screen.getByRole('tab', { name: 'Details' })).toBeDisabled();
  await user.click(screen.getByRole('button', { name: 'Replace files' }));
  expect(screen.getByLabelText('Replace manuscript files')).toBeVisible();
});
it('submits corrected metadata and source-bound results, keeping private review private', async () => {
  const submitPrepared = vi.fn(async () => ({
    ...prepared,
    status: { state: 'in_review' as const },
  }));
  const onSubmitted = vi.fn();
  renderWithApp(
    <SubmitPage
      initialSubmission={{ ...prepared, purpose: 'review_only' }}
      onSubmitted={onSubmitted}
    />,
    fakeApi({ submitPrepared }, author),
  );
  const user = userEvent.setup();
  await checkPdf(user);
  await user.clear(screen.getByLabelText('Title'));
  await user.type(screen.getByLabelText('Title'), 'Corrected title');
  await user.click(screen.getByRole('tab', { name: 'Authors' }));
  await user.type(screen.getByLabelText('Affiliation'), 'Example University');
  await user.type(screen.getByLabelText('ORCID'), '0000-0002-1825-0097');
  await user.type(
    screen.getByLabelText('Google Scholar'),
    'https://scholar.google.com/citations?user=ada',
  );
  await user.type(screen.getByLabelText('Website or other link'), 'https://example.edu/ada');
  await user.click(screen.getByRole('tab', { name: 'Submit' }));
  expect(screen.getByRole('checkbox', { name: /Make my results public/ })).not.toBeChecked();
  expect(screen.queryByText(/I confirm the manuscript details/)).toBeNull();
  await user.click(screen.getByRole('button', { name: 'Submit for review' }));
  expect(submitPrepared).toHaveBeenCalledWith(
    prepared.id,
    expect.objectContaining({
      version: 1,
      source_sha256: prepared.versions[0]!.archive.sha256,
      metadata: expect.objectContaining({
        title: 'Corrected title',
        authors: [
          {
            name: 'Ada Example',
            affiliation: 'Example University',
            orcid: '0000-0002-1825-0097',
            google_scholar: 'https://scholar.google.com/citations?user=ada',
            website: 'https://example.edu/ada',
          },
        ],
        purpose: 'review_only',
        make_public_after_acceptance: false,
      }),
      claims: expect.arrayContaining([expect.objectContaining({ id: prepared.extracted[0]!.id })]),
    }),
  );
  await waitFor(() => expect(onSubmitted).toHaveBeenCalledOnce());
});
it('asks anonymous visitors to sign in', async () => {
  renderWithApp(<SubmitPage />, fakeApi());
  expect(await screen.findByText('Sign in to submit your work.')).toBeVisible();
});

it.each([true, false])(
  'applies the final publication choice (%s) to typed conjectures',
  async (publish) => {
    const createSubmission = vi.fn(async () => draft);
    renderWithApp(<SubmitPage />, fakeApi({ createSubmission }, author));
    const user = userEvent.setup();
    await user.click(await screen.findByRole('button', { name: /No manuscript file/ }));
    expect(screen.queryByLabelText('Manuscript format')).toBeNull();
    expect(screen.queryByLabelText('Type of work')).toBeNull();
    await user.type(screen.getByLabelText('Title'), 'A finite graph conjecture');
    await user.type(
      screen.getByLabelText('Conjecture statement'),
      'Every graph in this family is connected.',
    );
    await user.click(screen.getByRole('tab', { name: 'Authors' }));
    await user.type(screen.getByLabelText('Name'), 'Ada Example');
    await user.click(screen.getByRole('tab', { name: 'Review & submit' }));
    const publication = screen.getByRole('checkbox', { name: /Make my results public/ });
    expect(publication).toBeChecked();
    if (!publish) await user.click(publication);
    await user.click(screen.getByRole('button', { name: 'Prepare conjecture PDF' }));
    expect(createSubmission).toHaveBeenCalledWith(
      expect.objectContaining({
        kind: 'conjecture',
        purpose: publish ? 'publication' : 'review_only',
        make_public_after_acceptance: publish,
        title: 'A finite graph conjecture',
        typed_conjecture: expect.objectContaining({
          statement: 'Every graph in this family is connected.',
        }),
        authors: [{ name: 'Ada Example' }],
      }),
    );
  },
);

it.each([true, false])(
  'uses the last-step publication choice (%s) for an uploaded manuscript',
  async (publish) => {
    const submitPrepared = vi.fn(async () => ({
      ...prepared,
      status: { state: 'in_review' as const },
    }));
    renderWithApp(<SubmitPage initialSubmission={prepared} />, fakeApi({ submitPrepared }, author));
    const user = userEvent.setup();
    await checkPdf(user);
    await user.click(screen.getByRole('tab', { name: 'Submit' }));
    const publication = screen.getByRole('checkbox', { name: /Make my results public/ });
    expect(publication).toBeChecked();
    expect(publication).toHaveAccessibleDescription(/Uncheck this box if you only want a review/);
    if (!publish) await user.click(publication);
    await user.click(screen.getByRole('button', { name: 'Submit for review' }));
    expect(submitPrepared).toHaveBeenCalledWith(
      prepared.id,
      expect.objectContaining({
        metadata: expect.objectContaining({
          purpose: publish ? 'publication' : 'review_only',
          make_public_after_acceptance: publish,
        }),
      }),
    );
  },
);
