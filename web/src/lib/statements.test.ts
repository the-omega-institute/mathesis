import { describe, expect, it } from 'vitest';
import { resultLabel } from './statements';

describe('result names', () => {
  it('preserves manuscript numbers and names', () => {
    expect(
      resultLabel({
        id: 'C4',
        kind: 'theorem',
        label: 'Theorem 2.1 [thm:clique]',
        section: 'Clique numbers',
      }),
    ).toBe('Theorem 2.1');
  });

  it('keeps the result type when its manuscript number is unavailable', () => {
    expect(
      resultLabel({
        id: 'C4',
        kind: 'theorem',
        label: 'Theorem [thm:clique]',
        section: 'The exact clique number',
      }),
    ).toBe('Theorem');
    expect(
      resultLabel({
        id: 'C1',
        kind: 'theorem',
        label: 'Theorem [thm:main]',
        section: 'Introduction',
      }),
    ).toBe('Theorem');
    expect(resultLabel({ id: 'C7', kind: 'theorem', label: 'Theorem [thm:bound]' })).toBe(
      'Theorem',
    );
  });
});
