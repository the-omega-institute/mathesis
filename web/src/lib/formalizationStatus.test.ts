import { describe, expect, it } from 'vitest';
import { accepted } from '../test/fixtures';
import { formalizationStatus } from './formalizationStatus';

describe('formalization status', () => {
  it('distinguishes a proposal from an admitted target-preparation request', () => {
    expect(formalizationStatus(accepted, 'C1').label).toBe('Awaiting your approval');
    expect(
      formalizationStatus(
        {
          ...accepted,
          formalization: {
            items: [{ ...accepted.formalization.items[0]!, state: { state: 'approved' } }],
          },
        },
        'C1',
      ).label,
    ).toBe('Queued for target preparation');
    expect(formalizationStatus(accepted, 'C2').label).toBe('Not scheduled');
  });

  it('ignores targets from an earlier source and distinguishes targets from proofs', () => {
    const target = {
      claim: 'C2',
      version: accepted.versions.at(-1)!.number,
      claims_revision: accepted.claims_revision,
      lean: 'def wishpool_target_prop : Prop := True',
      digest: 'a'.repeat(64),
      toolchain: 'Lean',
      reading: 'True',
      created_at: accepted.created_at,
      response: {
        state: 'confirmed' as const,
        author: accepted.submitter,
        at: accepted.created_at,
      },
    };
    expect(formalizationStatus({ ...accepted, lean_statements: [target] }, 'C2').label).toBe(
      'Target confirmed · proof not yet verified',
    );
    expect(
      formalizationStatus(
        { ...accepted, lean_statements: [{ ...target, claims_revision: 99 }] },
        'C2',
      ).label,
    ).toBe('Not scheduled');
  });
});
