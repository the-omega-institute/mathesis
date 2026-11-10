import { render, screen, within } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { ReviewOverview } from './ReviewOverview';
import { accepted, submission } from '../test/fixtures';
import { feedbackLetter, queuedRound, refereeRound } from '../test/referee';

describe('ReviewOverview', () => {
  it('summarizes a completed review and the next formalization state', () => {
    render(
      <ReviewOverview submission={accepted} round={refereeRound} letters={[feedbackLetter]} />,
    );
    const overview = screen.getByRole('region', { name: 'Review overview' });
    expect(within(overview).getByText('Referee report').nextElementSibling).toHaveTextContent(
      'Complete',
    );
    expect(within(overview).getByText('Source check').nextElementSibling).toHaveTextContent(
      'Complete',
    );
    expect(within(overview).getByText('Author feedback').nextElementSibling).toHaveTextContent(
      'Delivered',
    );
    expect(within(overview).getByText('Publication').nextElementSibling).toHaveTextContent(
      'Accepted',
    );
    expect(within(overview).getByText('Formalization').nextElementSibling).toHaveTextContent(
      '1 awaiting author approval',
    );
  });

  it('shows waiting states without implying that a CMA execution is a completed review', () => {
    render(<ReviewOverview submission={submission} round={queuedRound} />);
    const overview = screen.getByRole('region', { name: 'Review overview' });
    expect(within(overview).getAllByText('Waiting', { selector: '.badge' })).toHaveLength(2);
    expect(within(overview).getAllByText('In progress', { selector: '.badge' })).toHaveLength(2);
    expect(
      within(overview).getByText(/Execution history records runs and retries/),
    ).toBeInTheDocument();
  });
});

it('keeps awaiting consent separate from queued work and verified proofs', () => {
  render(
    <ReviewOverview
      submission={{
        ...accepted,
        formalization: {
          items: [
            ...accepted.formalization.items,
            {
              claim: 'C2',
              reason: '',
              state: { state: 'in_progress' },
              updated_at: accepted.updated_at,
            },
            {
              claim: 'C4',
              reason: '',
              state: {
                state: 'verified',
                artifact: {
                  repository: 'https://github.com/example/proof',
                  commit: 'a'.repeat(40),
                  declarations: ['proof'],
                },
                axioms: [],
              },
              updated_at: accepted.updated_at,
            },
          ],
        },
      }}
      round={refereeRound}
      letters={[feedbackLetter]}
    />,
  );
  const state = screen.getByText('Formalization').nextElementSibling;
  expect(state).toHaveTextContent('1 awaiting author approval');
  expect(state).toHaveTextContent('1 in progress');
  expect(state).toHaveTextContent('1 lean verified');
});
