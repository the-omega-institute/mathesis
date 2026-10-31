import { describe, expect, it } from 'vitest';
import { submission, accepted } from '../test/fixtures';
import { refereeRound } from '../test/referee';
import {
  formalizationAdvice,
  formalizationAdviceText,
  legacyFormalizationStatus,
} from './formalizationAdvice';

describe('author formalization advice', () => {
  it('shows selected proved results without inventing recommendations for the other statements', () => {
    const round = {
      ...refereeRound,
      formalization_suggestions: [
        {
          claim: 'C1',
          feasibility: 'ready' as const,
          plan: 'Formalize the reusable induction lemma.',
        },
        { claim: 'C3', feasibility: 'ready' as const, plan: 'This is an open question.' },
        { claim: 'missing', feasibility: 'ready' as const, plan: 'Another version.' },
      ],
    };
    const plans = formalizationAdvice(submission, round);
    expect(plans.map(({ claim }) => claim.id)).toEqual(['C1']);
    const text = formalizationAdviceText({ ...submission, purpose: 'review_only' }, round);
    expect(text).toContain('Theorem 1.1');
    expect(text).toContain('does not schedule public formalization work');
    expect(text).not.toContain('verified formal proof');
  });
});

it('reads old formalization records without inventing a proof or an active attempt', () => {
  const paper = {
    ...accepted,
    analysis_visibility: 'public' as const,
    formalization: {
      items: [
        {
          claim: 'C1',
          reason: 'Reusable lemma',
          state: { state: 'approved' as const },
          updated_at: accepted.updated_at,
        },
      ],
    },
  };
  expect(legacyFormalizationStatus(paper, 'C1')).toBe(
    'The exact Lean statement is being prepared.',
  );
  expect(legacyFormalizationStatus({ ...paper, analysis_visibility: 'private' }, 'C1')).toContain(
    'paused',
  );
  expect(legacyFormalizationStatus(paper, 'missing')).toBe('');
});
