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

  it('distinguishes legacy unnamed results without inventing manuscript numbering', () => {
    expect(
      resultLabel({
        id: 'C4',
        kind: 'theorem',
        label: 'Theorem [thm:clique]',
        section: 'The exact clique number',
      }),
    ).toBe('Theorem: The exact clique number (C4)');
    expect(resultLabel({ id: 'C7', kind: 'theorem', label: 'Theorem [thm:bound]' })).toBe(
      'Theorem C7',
    );
  });
});
