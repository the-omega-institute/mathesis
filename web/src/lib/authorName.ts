/*
 * One written form of an author name, and one rule for matching it. This is the
 * Layer 2 rule of `api/crates/layer2-core/src/model/author_name.rs`, for names
 * the web holds itself; both are tested against `api/fixtures/author-names.json`.
 * A matching name never identifies a researcher.
 */

/**
 * A token showing that a single comma does not separate a family name from
 * given names: generational suffixes, name lists, degrees and company forms.
 */
const NOT_A_PERSONAL_NAME: ReadonlySet<string> = new Set([
  'jr',
  'sr',
  'jnr',
  'snr',
  'junior',
  'senior',
  'ii',
  'iii',
  'iv',
  'and',
  'et',
  'phd',
  'ph.d',
  'dphil',
  'inc',
  'ltd',
  'llc',
]);

/** Latin, Greek and Cyrillic letters; family-first scripts such as CJK are never reordered. */
const LETTER = String.raw`(?=\p{Alphabetic})[A-Za-zÀ-ɏͰ-ϿЀ-ԯḀ-ỿ]`;
const HAS_LETTER = new RegExp(LETTER, 'u');
const NAME_TEXT = new RegExp(`^(?:${LETTER}|[ ,.'\\u2019-])*$`, 'u');

function excluded(token: string): boolean {
  // A bare "V" may be the fifth of a name; "V." is an initial.
  if (token === 'v' || token === 'V') return true;
  const word = token.endsWith('.') ? token.slice(0, -1) : token;
  return NOT_A_PERSONAL_NAME.has(word.replace(/[A-Z]/g, (letter) => letter.toLowerCase()));
}

/** The canonical written form, "Given Family"; other names are returned as stored. */
export function canonicalAuthorName(name: string): string {
  const parts = name.split(',');
  if (parts.length !== 2 || !NAME_TEXT.test(name)) return name;
  const [family, given] = parts.map((part) => part.replace(/^ +| +$/g, '')) as [string, string];
  if (!HAS_LETTER.test(family) || !HAS_LETTER.test(given)) return name;
  if ([...family.split(' '), ...given.split(' ')].some(excluded)) return name;
  return `${given} ${family}`;
}

/** A literal, case-insensitive substring of the name as written or in canonical form. */
export function authorNameMatches(name: string, query: string): boolean {
  return (
    name.toLowerCase().includes(query.toLowerCase()) ||
    canonicalAuthorName(name).toLowerCase().includes(canonicalAuthorName(query).toLowerCase())
  );
}
