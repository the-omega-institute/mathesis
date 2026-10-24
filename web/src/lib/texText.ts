/**
 * Plain text for a short TeX fragment such as an author name read from
 * `\author{...}`: accent commands and ligature macros become Unicode, braces
 * and ties disappear. Mathematics is left alone; this is for names and
 * affiliations, not statements.
 */
const LIGATURES: Readonly<Record<string, string>> = {
  ae: 'æ',
  AE: 'Æ',
  oe: 'œ',
  OE: 'Œ',
  o: 'ø',
  O: 'Ø',
  aa: 'å',
  AA: 'Å',
  ss: 'ß',
  l: 'ł',
  L: 'Ł',
  i: 'ı',
  j: 'ȷ',
  dh: 'ð',
  DH: 'Ð',
  th: 'þ',
  TH: 'Þ',
};
const ACCENTS: Readonly<Record<string, string>> = {
  "'": '́',
  '`': '̀',
  '^': '̂',
  '"': '̈',
  '~': '̃',
  '=': '̄',
  '.': '̇',
  u: '̆',
  v: '̌',
  H: '̋',
  c: '̧',
  k: '̨',
  r: '̊',
  b: '̱',
  d: '̣',
};

export function texToText(source: string): string {
  let text = source;
  // Accents: \'e  \'{e}  {\'e}  \c{c}  \v{s}  \"{o}
  text = text.replace(
    /\\([`'^"~=.uvHckrbd])\s*(?:\{\s*(\\?[A-Za-z])\s*\}|(\\?[A-Za-z]))/g,
    (_m, accent: string, braced?: string, bare?: string) => {
      const raw = braced ?? bare ?? '';
      // `\'\i` and `\"\o` accent a letter macro; a plain letter stays itself.
      const base = raw.startsWith('\\') ? (LIGATURES[raw.slice(1)] ?? raw.slice(1)) : raw;
      return (base + ACCENTS[accent]).normalize('NFC');
    },
  );
  // Ligature and letter macros: \ae \o \ss \l, terminated by a non-letter or {}.
  text = text.replace(
    /\\(ae|AE|oe|OE|aa|AA|ss|dh|DH|th|TH|o|O|l|L|i|j)(?:\{\})?(?![A-Za-z])\s?/g,
    (_m, name: string) => LIGATURES[name] ?? _m,
  );
  text = text.replace(/~/g, ' ').replace(/[{}]/g, '');
  return text.replace(/\s+/g, ' ').trim();
}
