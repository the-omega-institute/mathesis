import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { authorNameMatches, canonicalAuthorName } from './authorName';

/** The cases of the Layer 2 rule (`author_name.rs`), so both implementations agree. */
const cases = JSON.parse(readFileSync('../api/fixtures/author-names.json', 'utf8')) as {
  canonical: { stored: string; canonical: string }[];
  matching: { stored: string; query: string; matches: boolean }[];
};

describe('author names', () => {
  it('rewrites only the single-comma "Family, Given" form', () => {
    expect(cases.canonical.length).toBeGreaterThan(40);
    for (const { stored, canonical } of cases.canonical) {
      expect(canonicalAuthorName(stored), stored).toBe(canonical);
      expect(canonicalAuthorName(canonical), `${stored} is not stable`).toBe(canonical);
    }
    expect(canonicalAuthorName('Ma, Haobo')).toBe('Haobo Ma');
    expect(canonicalAuthorName('de la Cruz, Ana')).toBe('Ana de la Cruz');
    expect(canonicalAuthorName('  Ma,  Haobo ')).toBe('Haobo Ma');
    for (const stored of ['Smith, John, Jr.', 'John Smith, Jr.', 'Euclid', '张文林', '张, 文林']) {
      expect(canonicalAuthorName(stored)).toBe(stored);
    }
  });

  it('matches a name in either written form, as the archive API does', () => {
    expect(cases.matching.length).toBeGreaterThan(20);
    for (const { stored, query, matches } of cases.matching) {
      expect(authorNameMatches(stored, query), `${stored} / ${query}`).toBe(matches);
    }
    for (const stored of ['Ma, Haobo', 'Haobo Ma']) {
      expect(authorNameMatches(stored, 'Haobo Ma')).toBe(true);
      expect(authorNameMatches(stored, 'Ma, Haobo')).toBe(true);
    }
  });
});
