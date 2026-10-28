import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { strFromU8, unzipSync } from 'fflate';
import { expect, it, vi } from 'vitest';
import { ApiError } from '../api/client';
import { author, draft } from '../test/fixtures';
import { fakeApi, renderWithApp } from '../test/render';
import { SubmitPage } from './Submit';

async function fillMetadata(user: ReturnType<typeof userEvent.setup>) {
  await user.type(await screen.findByLabelText('Title'), 'A precise mathematical result');
  await user.type(screen.getByLabelText('Abstract'), 'We establish the stated bound.');
  await user.click(screen.getByRole('tab', { name: 'Authors' }));
  await user.type(screen.getByLabelText('Name'), 'Ada Example');
  await user.type(screen.getByLabelText('Affiliation'), 'Example University');
  await user.click(screen.getByRole('tab', { name: 'Files' }));
}
async function finish(user: ReturnType<typeof userEvent.setup>) {
  await user.click(screen.getByRole('tab', { name: 'Review & submit' }));
  await user.click(screen.getByRole('checkbox', { name: /I agree that the source/ }));
  await user.click(screen.getByRole('button', { name: 'Upload manuscript' }));
}
it('asks anonymous visitors to sign in', async () => {
  renderWithApp(<SubmitPage />, fakeApi());
  expect(await screen.findByText('Sign in to submit your work.')).toBeVisible();
});
it('keeps section values while navigating accessible horizontal tabs', async () => {
  renderWithApp(<SubmitPage />, fakeApi({}, author));
  const user = userEvent.setup();
  await fillMetadata(user);
  expect(screen.getAllByRole('tabpanel')).toHaveLength(1);
  expect(screen.queryByLabelText('Title')).toBeNull();
  await user.click(screen.getByRole('tab', { name: 'Manuscript' }));
  expect(screen.getByLabelText('Title')).toHaveValue('A precise mathematical result');
  await user.keyboard('{ArrowRight}');
  expect(screen.getByRole('tab', { name: 'Authors' })).toHaveFocus();
  expect(screen.getByLabelText('Name')).toHaveValue('Ada Example');
});
it.each(['paper.tex', 'paper.zip', 'paper.tar.gz', 'paper.pdf'])(
  'uploads %s with author metadata and explicit consent',
  async (name) => {
    const createSubmission = vi.fn(async () => draft);
    renderWithApp(<SubmitPage />, fakeApi({ createSubmission }, author));
    const user = userEvent.setup();
    await fillMetadata(user);
    const file = new File(['manuscript'], name);
    await user.upload(screen.getByLabelText('Choose manuscript files'), file);
    await user.click(screen.getByRole('tab', { name: 'Review & submit' }));
    expect(screen.getByRole('checkbox', { name: /I agree that the source/ })).not.toBeChecked();
    await user.click(screen.getByRole('button', { name: 'Upload manuscript' }));
    expect(createSubmission).not.toHaveBeenCalled();
    expect(screen.getByRole('alert')).toHaveTextContent('Agree to third-party AI review');
    await finish(user);
    expect(createSubmission).toHaveBeenCalledWith(
      expect.objectContaining({
        title: 'A precise mathematical result',
        authors: [{ name: 'Ada Example', affiliation: 'Example University' }],
        purpose: 'publication',
        processing_consent: 'ai-review-2026-10-11',
      }),
      file,
    );
    await waitFor(() => expect(window.location.pathname).toBe('/submissions/sub-draft'));
  },
);
it('bundles multiple TeX files and bibliography without dropping files', async () => {
  const createSubmission = vi.fn(async () => draft);
  renderWithApp(<SubmitPage />, fakeApi({ createSubmission }, author));
  const user = userEvent.setup();
  await fillMetadata(user);
  await user.upload(screen.getByLabelText('Choose manuscript files'), [
    new File(['main'], 'main.tex'),
    new File(['proof'], 'proof.tex'),
    new File(['refs'], 'refs.bib'),
  ]);
  await finish(user);
  await waitFor(() => expect(createSubmission).toHaveBeenCalledOnce());
  const file = (createSubmission.mock.calls as unknown as [unknown, File][])[0]![1];
  const contents = unzipSync(new Uint8Array(await file.arrayBuffer()));
  expect(Object.keys(contents)).toEqual(['main.tex', 'proof.tex', 'refs.bib']);
  expect(strFromU8(contents['proof.tex']!)).toBe('proof');
});
it('keeps private reviews private and reports upload errors without losing input', async () => {
  const createSubmission = vi
    .fn()
    .mockRejectedValueOnce(
      new ApiError({ status: 422, code: 'invalid', detail: 'PDF text could not be read.' }),
    )
    .mockResolvedValue(draft);
  renderWithApp(<SubmitPage initialIntake="review" />, fakeApi({ createSubmission }, author));
  const user = userEvent.setup();
  await fillMetadata(user);
  await user.upload(
    screen.getByLabelText('Choose manuscript files'),
    new File(['pdf'], 'paper.pdf'),
  );
  await finish(user);
  expect(await screen.findByRole('alert')).toHaveTextContent('PDF text could not be read.');
  expect(screen.queryByRole('checkbox', { name: /Publish/ })).toBeNull();
  await user.click(screen.getByRole('button', { name: 'Upload manuscript' }));
  expect(createSubmission).toHaveBeenLastCalledWith(
    expect.objectContaining({
      purpose: 'review_only',
      make_public_after_acceptance: false,
      open_to_contributors: false,
    }),
    expect.any(File),
  );
});
it('submits a typed conjecture without a file', async () => {
  const createSubmission = vi.fn(async () => draft);
  renderWithApp(<SubmitPage />, fakeApi({ createSubmission }, author));
  const user = userEvent.setup();
  await user.selectOptions(await screen.findByLabelText('Type of work'), 'conjecture');
  await user.selectOptions(screen.getByLabelText('Manuscript format'), 'type');
  await user.type(screen.getByLabelText('Conjecture statement'), 'Every $n>1$ has the property.');
  await fillMetadata(user);
  expect(screen.queryByLabelText('Choose manuscript files')).toBeNull();
  await finish(user);
  expect(createSubmission).toHaveBeenCalledWith(
    expect.objectContaining({
      kind: 'conjecture',
      typed_conjecture: expect.objectContaining({ statement: 'Every $n>1$ has the property.' }),
    }),
    undefined,
  );
});
it('returns to the invalid section and preserves metadata', async () => {
  const createSubmission = vi.fn();
  renderWithApp(<SubmitPage />, fakeApi({ createSubmission }, author));
  const user = userEvent.setup();
  await fillMetadata(user);
  await user.click(screen.getByRole('tab', { name: 'Manuscript' }));
  await user.type(screen.getByLabelText('DOI (optional)'), 'invalid');
  await user.click(screen.getByRole('tab', { name: 'Files' }));
  await user.upload(screen.getByLabelText('Choose manuscript files'), new File(['x'], 'paper.tex'));
  await finish(user);
  expect(screen.getByRole('tab', { name: 'Manuscript' })).toHaveAttribute('aria-selected', 'true');
  expect(screen.getByRole('alert')).toHaveTextContent('not a DOI');
  expect(screen.getByLabelText('Title')).toHaveValue('A precise mathematical result');
  expect(createSubmission).not.toHaveBeenCalled();
});
