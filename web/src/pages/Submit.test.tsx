import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { strFromU8, unzipSync } from 'fflate';
import { describe, expect, it, vi } from 'vitest';
import { ApiError } from '../api/client';
import { author, draft } from '../test/fixtures';
import { fakeApi, renderWithApp } from '../test/render';
import { SubmitPage } from './Submit';

describe('SubmitPage', () => {
  it('asks anonymous visitors to sign in', async () => {
    renderWithApp(<SubmitPage />, fakeApi());
    expect(await screen.findByText('Sign in to submit your work.')).toBeInTheDocument();
  });

  it('uploads the source with the metadata and opens the draft', async () => {
    const createSubmission = vi.fn(async () => draft);
    renderWithApp(<SubmitPage />, fakeApi({ createSubmission }, author));
    const user = userEvent.setup();

    const file = new File(['\\documentclass{amsart}'], 'paper.tar.gz', {
      type: 'application/gzip',
    });
    await user.upload(await screen.findByLabelText('LaTeX source (required)'), file);
    expect(
      screen.getByText('If the paper already has one, e.g. 10.48550/arXiv.2609.33421'),
    ).toBeInTheDocument();
    expect(screen.queryByLabelText('arXiv identifier (optional)')).toBeNull();
    await user.click(screen.getByRole('radio', { name: /^Substantial/ }));
    await user.type(screen.getByLabelText('MSC 2020 codes (optional)'), '11b83 05d10');
    await user.type(
      screen.getByLabelText('DOI (optional)'),
      'https://doi.org/10.48550/arXiv.2609.33421',
    );
    await user.click(screen.getByRole('checkbox', { name: /volunteer contributors/ }));
    if (
      !(screen.getByRole('checkbox', { name: /I agree that the source/ }) as HTMLInputElement)
        .checked
    )
      await user.click(screen.getByRole('checkbox', { name: /I agree that the source/ }));
    await user.click(screen.getByRole('button', { name: 'Upload and read statements' }));

    expect(createSubmission).toHaveBeenCalledWith(
      {
        kind: 'paper',
        purpose: 'publication',
        review_scope: '',
        processing_consent: 'ai-review-2026-10-11',
        make_public_after_acceptance: true,
        ai_disclosure: {
          level: 'substantial',
          statement: 'AI systems produced substantial parts of the proofs or text.',
        },
        msc: ['11B83', '05D10'],
        doi: '10.48550/arXiv.2609.33421',
        open_to_contributors: true,
      },
      file,
    );
    await waitFor(() => expect(window.location.pathname).toBe('/submissions/sub-draft'));
  });

  it('rejects an invalid DOI before uploading', async () => {
    const createSubmission = vi.fn(async () => draft);
    renderWithApp(<SubmitPage />, fakeApi({ createSubmission }, author));
    const user = userEvent.setup();
    await user.upload(
      await screen.findByLabelText('LaTeX source (required)'),
      new File(['x'], 'paper.tex', { type: 'application/x-tex' }),
    );
    await user.type(screen.getByLabelText('DOI (optional)'), '2609.33421');
    if (
      !(screen.getByRole('checkbox', { name: /I agree that the source/ }) as HTMLInputElement)
        .checked
    )
      await user.click(screen.getByRole('checkbox', { name: /I agree that the source/ }));
    await user.click(screen.getByRole('button', { name: 'Upload and read statements' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('"2609.33421" is not a DOI');
    expect(createSubmission).not.toHaveBeenCalled();
  });

  it('keeps an optional PDF reading copy beside the LaTeX source', async () => {
    const createSubmission = vi.fn(async () => draft);
    renderWithApp(<SubmitPage />, fakeApi({ createSubmission }, author));
    const user = userEvent.setup();
    await user.upload(
      await screen.findByLabelText('LaTeX source (required)'),
      new File(['\\documentclass{article}'], 'paper.tex', { type: 'application/x-tex' }),
    );
    await user.upload(
      screen.getByLabelText('PDF manuscript or reading copy (optional)'),
      new File(['%PDF-1.7'], 'paper.pdf', { type: 'application/pdf' }),
    );
    await user.click(screen.getByRole('checkbox', { name: /I agree that the source/ }));
    await user.click(screen.getByRole('button', { name: 'Upload and read statements' }));

    const source = (createSubmission.mock.calls as unknown as [unknown, File][])[0]?.[1];
    expect(createSubmission).toHaveBeenCalled();
    expect(source).toBeInstanceOf(File);
    expect((source as File).name).toBe('mathesis-source.zip');
    const files = unzipSync(new Uint8Array(await (source as File).arrayBuffer()));
    const manifest = JSON.parse(strFromU8(files['mathesis-evidence.json']!)) as {
      files: { path: string; description: string; kind: string }[];
    };
    expect(manifest.files).toContainEqual({
      path: 'evidence/paper.pdf',
      description: 'Author-provided PDF reading copy.',
      kind: 'supplement',
      claims: [],
    });
  });

  it('lets an author supply names omitted from an anonymous source without linking accounts', async () => {
    const createSubmission = vi.fn(async () => draft);
    renderWithApp(<SubmitPage />, fakeApi({ createSubmission }, author));
    const user = userEvent.setup();
    const file = new File(['\\documentclass{article}'], 'anonymous.tex');
    await user.upload(await screen.findByLabelText('LaTeX source (required)'), file);
    await user.type(screen.getByLabelText('Name'), 'Ada Example');
    await user.type(screen.getByLabelText('Affiliation'), 'University of Example');
    await user.type(screen.getByLabelText('ORCID'), '0000-0002-1825-0097');
    await user.type(
      screen.getByLabelText('Google Scholar'),
      'https://scholar.google.com/citations?user=abc',
    );
    await user.type(screen.getByLabelText('Personal website'), 'https://example.edu/~ada');
    await user.click(screen.getByRole('button', { name: 'Add another author' }));
    await user.type(screen.getAllByLabelText('Name')[1]!, 'Benoit Example');
    await user.click(screen.getByRole('checkbox', { name: /I agree that the source/ }));
    await user.click(screen.getByRole('button', { name: 'Upload and read statements' }));
    expect(createSubmission).toHaveBeenCalledWith(
      expect.objectContaining({
        authors: [
          {
            name: 'Ada Example',
            affiliation: 'University of Example',
            orcid: '0000-0002-1825-0097',
            google_scholar: 'https://scholar.google.com/citations?user=abc',
            website: 'https://example.edu/~ada',
          },
          { name: 'Benoit Example' },
        ],
      }),
      file,
    );
  });

  it('refuses a PDF before uploading and shows the server’s detail on failure', async () => {
    const createSubmission = vi.fn(async () => {
      throw new ApiError({
        status: 422,
        code: 'invalid',
        detail: 'no main file: the archive has no .tex file with \\documentclass',
      });
    });
    renderWithApp(<SubmitPage />, fakeApi({ createSubmission }, author));
    const user = userEvent.setup({ applyAccept: false });
    const input = await screen.findByLabelText('LaTeX source (required)');

    await user.upload(input, new File(['%PDF'], 'paper.pdf', { type: 'application/pdf' }));
    if (
      !(screen.getByRole('checkbox', { name: /I agree that the source/ }) as HTMLInputElement)
        .checked
    )
      await user.click(screen.getByRole('checkbox', { name: /I agree that the source/ }));
    await user.click(screen.getByRole('button', { name: 'Upload and read statements' }));
    expect(await screen.findByRole('alert')).toHaveTextContent(/Choose a \.tex file/);
    expect(createSubmission).not.toHaveBeenCalled();

    await user.upload(input, new File(['x'], 'paper.zip', { type: 'application/zip' }));
    if (
      !(screen.getByRole('checkbox', { name: /I agree that the source/ }) as HTMLInputElement)
        .checked
    )
      await user.click(screen.getByRole('checkbox', { name: /I agree that the source/ }));
    await user.click(screen.getByRole('button', { name: 'Upload and read statements' }));
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'no main file: the archive has no .tex file with \\documentclass',
    );
  });
  it('uses the same upload for a short note with the publication checkbox off', async () => {
    const createSubmission = vi.fn(async () => draft);
    renderWithApp(<SubmitPage />, fakeApi({ createSubmission }, author));
    const user = userEvent.setup();
    await user.click(await screen.findByRole('radio', { name: 'Short note' }));
    const publication = screen.getByRole('checkbox', {
      name: 'Make public after acceptance (optional)',
    });
    expect(publication).toBeChecked();
    await user.click(publication);
    const source = new File(['source'], 'note.tex', { type: 'application/x-tex' });
    await user.upload(screen.getByLabelText('LaTeX source (required)'), source);
    if (
      !(screen.getByRole('checkbox', { name: /I agree that the source/ }) as HTMLInputElement)
        .checked
    )
      await user.click(screen.getByRole('checkbox', { name: /I agree that the source/ }));
    await user.click(screen.getByRole('button', { name: 'Upload and read statements' }));
    expect(createSubmission).toHaveBeenCalledWith(
      expect.objectContaining({ kind: 'note', make_public_after_acceptance: false }),
      source,
    );
  });

  it('submits typed conjectures with TeX preview through the same endpoint without a file', async () => {
    const createSubmission = vi.fn(async () => draft);
    renderWithApp(<SubmitPage />, fakeApi({ createSubmission }, author));
    const user = userEvent.setup();
    await user.click(await screen.findByRole('radio', { name: 'Conjecture' }));
    await user.click(screen.getByRole('radio', { name: 'Type it' }));
    expect(screen.queryByLabelText('LaTeX source (required)')).not.toBeInTheDocument();
    const manuscript = screen.getByRole('region', { name: 'Manuscript' });
    const choices = screen.getByRole('group', { name: 'Type of work (defaults to Paper)' });
    const title = screen.getByLabelText('Title (required)');
    const statement = screen.getByLabelText('Statement (required)');
    expect(manuscript).toContainElement(choices);
    expect(choices.compareDocumentPosition(title) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(title).toBeRequired();
    expect(statement).toBeRequired();
    const authorName = screen.getByLabelText('Author (required)');
    expect(authorName).toBeRequired();
    expect(authorName.closest('.field')).toHaveTextContent(
      'We use this name to attribute your conjecture.',
    );
    expect(manuscript).toHaveTextContent('We use the title to identify your conjecture.');
    expect(manuscript).toHaveTextContent('We review this statement to assess the conjecture.');
    const optional = screen.getByRole('region', { name: 'Optional information and preferences' });
    expect(optional).toContainElement(screen.getByLabelText('Background (optional)'));
    expect(optional).toContainElement(screen.getByLabelText('Origin (optional)'));
    await user.click(screen.getByRole('checkbox', { name: /I agree that the source/ }));
    await user.click(screen.getByRole('button', { name: 'Upload and read statements' }));
    expect(screen.getByRole('alert')).toHaveTextContent('Give a title, statement and author name.');
    expect(createSubmission).not.toHaveBeenCalled();
    await user.type(title, 'An open bound');
    await user.click(screen.getByRole('button', { name: 'Upload and read statements' }));
    expect(screen.getByRole('alert')).toHaveTextContent('Give a title, statement and author name.');
    expect(createSubmission).not.toHaveBeenCalled();
    await user.type(statement, 'Every $n > 1$ has the property.');
    await user.clear(authorName);
    await user.click(screen.getByRole('button', { name: 'Upload and read statements' }));
    expect(screen.getByRole('alert')).toHaveTextContent('Give a title, statement and author name.');
    expect(createSubmission).not.toHaveBeenCalled();
    await user.type(authorName, author.display_name);
    await user.type(screen.getByLabelText('Background (optional)'), 'Let $n$ be a natural number.');
    await user.type(screen.getByLabelText('Origin (optional)'), 'my own');
    const preview = screen.getByLabelText('Statement preview');
    expect(preview.querySelector('.katex')).not.toBeNull();
    expect(
      screen.getByRole('checkbox', { name: 'Make public after acceptance (optional)' }),
    ).toBeChecked();
    expect(screen.getByText('Reviews and letters are only shown to you.')).toBeInTheDocument();
    if (
      !(screen.getByRole('checkbox', { name: /I agree that the source/ }) as HTMLInputElement)
        .checked
    )
      await user.click(screen.getByRole('checkbox', { name: /I agree that the source/ }));
    await user.click(screen.getByRole('button', { name: 'Upload and read statements' }));
    expect(createSubmission).toHaveBeenCalledWith(
      expect.objectContaining({
        kind: 'conjecture',
        make_public_after_acceptance: true,
        typed_conjecture: {
          title: 'An open bound',
          statement: 'Every $n > 1$ has the property.',
          background: 'Let $n$ be a natural number.',
          origin: 'my own',
        },
        authors: [{ name: author.display_name, person: author.id }],
      }),
      undefined,
    );
  });
});

it('requires explicit AI processing consent, unchecked on a fresh upload', async () => {
  const createSubmission = vi.fn(async () => draft);
  const user = userEvent.setup();
  renderWithApp(<SubmitPage />, fakeApi({ createSubmission }, author));
  await user.upload(
    await screen.findByLabelText('LaTeX source (required)'),
    new File(['source'], 'paper.tex', { type: 'text/plain' }),
  );
  expect(screen.getByRole('checkbox', { name: /I agree that the source/ })).not.toBeChecked();
  await user.click(screen.getByRole('button', { name: 'Upload and read statements' }));
  expect(createSubmission).not.toHaveBeenCalled();
  expect(await screen.findByRole('alert')).toHaveTextContent(
    'Agree to third-party AI review before uploading.',
  );
});

it('requests a private review with a focus and no public contribution switches', async () => {
  const createSubmission = vi.fn(async () => draft);
  renderWithApp(<SubmitPage initialIntake="review" />, fakeApi({ createSubmission }, author));
  const user = userEvent.setup();
  const source = new File(['source'], 'review.tex', { type: 'text/plain' });
  await user.upload(await screen.findByLabelText('LaTeX source (required)'), source);
  expect(
    screen.queryByRole('checkbox', { name: 'Make public after acceptance (optional)' }),
  ).toBeNull();
  expect(screen.queryByRole('checkbox', { name: /volunteer contributors/ })).toBeNull();
  await user.type(
    screen.getByLabelText(/What should the review focus/),
    'Check the boundary case.',
  );
  await user.click(screen.getByRole('checkbox', { name: /I agree that the source/ }));
  await user.click(screen.getByRole('button', { name: 'Upload and read statements' }));
  expect(createSubmission).toHaveBeenCalledWith(
    expect.objectContaining({
      purpose: 'review_only',
      review_scope: 'Check the boundary case.',
      make_public_after_acceptance: false,
      open_to_contributors: false,
    }),
    source,
  );
});

it.each(['paper', 'note'] as const)(
  'explains the three required inputs and groups optional fields afterwards for a %s',
  async (kind) => {
    renderWithApp(<SubmitPage />, fakeApi({}, author));
    const source = await screen.findByLabelText('LaTeX source (required)');
    expect(screen.getByRole('radio', { name: 'Paper' })).toBeChecked();
    if (kind === 'note') await userEvent.click(screen.getByRole('radio', { name: 'Short note' }));
    const form = screen.getByRole('form', { name: 'Submit work' });
    const required = form.querySelector('.submission-manuscript')!;
    const choices = screen.getByRole('group', { name: 'Type of work (defaults to Paper)' });
    expect(required).toContainElement(choices);
    expect(choices.compareDocumentPosition(source) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    const optional = screen
      .getByRole('heading', { name: 'Optional information and preferences' })
      .closest('section')!;
    expect(source).toBeRequired();
    expect(required).toHaveTextContent(
      'We read the source to identify your statements and check their arguments.',
    );
    expect(required).toHaveTextContent('AI use in this work (required)');
    expect(required).toHaveTextContent(
      'We ask so readers understand how the mathematics and text were produced.',
    );
    expect(required).toHaveTextContent('Processing consent (required)');
    expect(required).toHaveTextContent(
      'We need your permission to send the manuscript to the review services that read and check it.',
    );
    expect(screen.getByRole('checkbox', { name: /Processing consent/ })).not.toBeChecked();
    expect(required.textContent?.match(/\(required\)/g)).toHaveLength(3);
    expect(
      required.compareDocumentPosition(optional) & Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
    for (const control of [
      'PDF manuscript or reading copy (optional)',
      'What should the review focus on? (optional)',
      'MSC 2020 codes (optional)',
      'DOI (optional)',
      'Attach evidence files (optional)',
    ]) {
      expect(optional).toContainElement(screen.getByLabelText(control));
    }
    expect(optional).toHaveTextContent('Authors (optional)');
    expect(optional).toHaveTextContent('Computational evidence & supplements (optional)');
    expect(
      screen.getByRole('heading', { name: 'What happens after you upload' }).closest('section'),
    ).toHaveTextContent(
      'Upload → you confirm the statement list → the review runs → the feedback letter arrives on this site → accepted work receives a WP record.',
    );
  },
);
