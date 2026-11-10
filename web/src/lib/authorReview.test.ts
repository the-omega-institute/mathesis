import { describe, expect, it } from 'vitest';
import { accepted, submission, version } from '../test/fixtures';
import { feedbackLetter, queuedRound, refereeFile, refereeRound } from '../test/referee';
import { authorReview } from './authorReview';

describe('author review delivery', () => {
  it('does not present a delivered letter as a final result while the decision is unsettled', () => {
    const review = authorReview(submission, refereeFile({ letters: [feedbackLetter] }));
    expect(review.letter).toBeUndefined();
    expect(review.status.label).toBe('Review in progress');
  });

  it.each([
    { rounds: [refereeRound], letters: [{ ...feedbackLetter, round: undefined }] },
    { rounds: [{ ...refereeRound, version: 2 }], letters: [feedbackLetter] },
    { rounds: [{ ...refereeRound, claims_revision: 99 }], letters: [feedbackLetter] },
  ])('does not attach a letter without a matching source and round', (file) => {
    expect(authorReview(accepted, refereeFile(file)).letter).toBeUndefined();
  });

  it('retains the delivered report while a follow-up on the same version runs', () => {
    const file = refereeFile({
      rounds: [refereeRound, { ...queuedRound, number: 2, response_ids: ['reply'] }],
      letters: [feedbackLetter],
    });
    const review = authorReview(accepted, file);
    expect(review.status.label).toBe('Follow-up review');
    expect(review.reportRound?.number).toBe(1);
    expect(review.letter).toEqual(feedbackLetter);
  });

  it('does not carry an old report forward after a revision', () => {
    const review = authorReview(
      { ...submission, versions: [version(1), version(2)] },
      refereeFile({ letters: [feedbackLetter] }),
    );
    expect(review.letter).toBeUndefined();
    expect(review.current).toBeUndefined();
  });

  it('does not turn private review completion into a publication decision', () => {
    const privatePaper = {
      ...submission,
      purpose: 'review_only' as const,
      status: { state: 'review_complete' as const },
    };
    const preparing = authorReview(privatePaper, refereeFile());
    expect(preparing.status.label).toBe('Review in progress');
    expect(preparing.status.description).toContain('does not include a publication decision');
    const ready = authorReview(privatePaper, refereeFile({ letters: [feedbackLetter] }));
    expect(ready.status.label).toBe('Review complete');
  });

  it('keeps a completed review valid when a later Lean probe fails', () => {
    const at = refereeRound.started_at;
    const review = authorReview(
      { ...submission, purpose: 'review_only', status: { state: 'review_complete' } },
      refereeFile({
        rounds: [
          {
            ...refereeRound,
            formal: {
              attempts: 1,
              state: { state: 'failed', at, reason: 'Interrupted', retryable: true },
            },
          },
        ],
        letters: [feedbackLetter],
      }),
    );
    expect(review.status.label).toBe('Review complete');
    expect(review.letter).toEqual(feedbackLetter);
  });
});
