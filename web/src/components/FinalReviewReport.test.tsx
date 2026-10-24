import { screen, within } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { FinalReviewReport } from './FinalReviewReport';
import { accepted, author, notAccepted, submission } from '../test/fixtures';
import { feedbackLetter, queuedRound, refereeFile } from '../test/referee';
import { fakeApi, renderWithApp } from '../test/render';

describe('FinalReviewReport', () => {
  it('presents one author-facing report with the decision and accepted statements', async () => {
    const api = fakeApi({}, author);
    const file = refereeFile({ letters: [{ ...feedbackLetter, round: 1 }] });
    renderWithApp(<FinalReviewReport submission={accepted} file={file} onChange={vi.fn()} />, api);

    const report = await screen.findByRole('region', { name: 'AI review report' });
    expect(report).toHaveTextContent('AI review');
    expect(report).toHaveTextContent('Accepted');
    expect(report).toHaveTextContent('Please clarify the base case');
    expect(report).toHaveTextContent('Theorem 1.1');
    expect(report).toHaveTextContent('Ada Author, Ben Second');
    expect(report).not.toHaveTextContent('Source audit');
    expect(report).not.toHaveTextContent('GPT Pro');
    expect(report).not.toHaveTextContent('Codex');
    expect(report).not.toHaveTextContent('CMA');
    expect(report).not.toHaveTextContent('Feedback letter');
    expect(
      within(report).getByRole('link', { name: 'Structured review data (JSON)', hidden: true }),
    ).toHaveAttribute('href', `/api/v1/submissions/${accepted.id}/review`);
    expect(within(report).getByRole('link', { name: 'Download report' })).toHaveAttribute(
      'href',
      `/api/v1/submissions/${accepted.id}/review.md`,
    );
  });

  it('puts a non-acceptance decision and revision action in the same report', async () => {
    const api = fakeApi({}, author);
    const file = refereeFile({ letters: [] });
    renderWithApp(
      <FinalReviewReport
        submission={notAccepted}
        file={file}
        onChange={vi.fn()}
        onRevise={vi.fn()}
      />,
      api,
    );

    const report = await screen.findByRole('region', { name: 'AI review report' });
    expect(report).toHaveTextContent('Not accepted');
    expect(report).toHaveTextContent('Statement C1 already follows from 1901.00001.');
    expect(report).toHaveTextContent('Upload a revised version');
  });

  it('uses a plain completion state when no delivery letter is available', async () => {
    const api = fakeApi({}, author);
    const file = refereeFile({ letters: [] });
    renderWithApp(
      <FinalReviewReport
        submission={{ ...submission, purpose: 'review_only', status: { state: 'review_complete' } }}
        file={file}
        onChange={vi.fn()}
      />,
      api,
    );

    const report = await screen.findByRole('region', { name: 'AI review report' });
    expect(report.querySelector('.final-report-status')).toHaveTextContent(
      'The assessment is complete and the final report is being prepared. Your private review does not include a publication decision.',
    );
    expect(report).not.toHaveTextContent('Source audit');
    expect(report).not.toHaveTextContent('Improvement suggestions');
  });

  it('keeps report downloads hidden while the review is still running', async () => {
    const api = fakeApi({}, author);
    const file = refereeFile({ rounds: [queuedRound] });
    renderWithApp(
      <FinalReviewReport submission={submission} file={file} onChange={vi.fn()} />,
      api,
    );

    const report = await screen.findByRole('region', { name: 'AI review report' });
    expect(report).toHaveTextContent('Review in progress');
    expect(
      within(report).queryByRole('link', { name: 'Structured review data (JSON)', hidden: true }),
    ).toBeNull();
    expect(within(report).queryByRole('link', { name: 'Download report' })).toBeNull();
  });
});

it('does not label claims without a completed audit as accepted results', async () => {
  const file = refereeFile({
    letters: [feedbackLetter],
    rounds: [{ ...queuedRound, audit: undefined }],
  });
  renderWithApp(
    <FinalReviewReport submission={accepted} file={file} onChange={vi.fn()} />,
    fakeApi({}, author),
  );
  const report = await screen.findByRole('region', { name: 'AI review report' });
  expect(report).toHaveTextContent('Accepted');
  expect(within(report).queryByRole('heading', { name: 'Accepted statements' })).toBeNull();
  expect(report).not.toHaveTextContent('Lean checked');
});
