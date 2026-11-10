import { describe, expect, it } from 'vitest';
import { paperSummary, publicPaper } from '../test/fixtures';
import { paperCitation, paperBibtex } from './citation';

describe('paperCitation', () => {
  it('includes the DOI and its resolver link', () => {
    const citation = paperCitation(paperSummary, 'https://wishpool.example');
    expect(citation).toContain(
      'doi:10.48550/arXiv.2609.33421 (https://doi.org/10.48550/arXiv.2609.33421).',
    );
    expect(citation).toContain('https://wishpool.example/papers/WP-2026-0001');
  });

  it('omits the DOI when the paper has none', () => {
    expect(
      paperCitation({ ...paperSummary, doi: undefined }, 'https://wishpool.example'),
    ).not.toContain('doi:');
  });
});

it('exports the containing work and exact result locator without inventing a journal', () => {
  const claim = { ...publicPaper.claims![0]!, label: 'Theorem (Summation formula) [thm:sum]' };
  const bib = paperBibtex(paperSummary, 'https://mathesis.example', { claim, version: 3 });
  expect(bib).toContain('@misc{WP-2026-0001-C1,');
  expect(bib).toContain('Theorem (Summation formula)');
  expect(bib).toContain('version 3');
  expect(bib).not.toContain('thm:sum');
  expect(bib).not.toContain('journal =');
  expect(bib).toContain('/papers/WP-2026-0001/results/C1');
  expect(bib).toContain('archivePrefix = {arXiv}');
});

it('keeps mathematical TeX and names intact while containing malformed braces and comments', () => {
  const bib = paperBibtex(
    {
      ...paperSummary,
      title: String.raw`Bounds in $\mathbb{R}$: 50% & a } malformed { title`,
      authors: [{ name: String.raw`Ren\'e Author` }],
    },
    'https://mathesis.example',
  );
  expect(bib).toContain(String.raw`$\mathbb{R}$`);
  expect(bib).toContain(String.raw`50\% \& a \} malformed \{ title`);
  expect(bib).toContain(String.raw`Ren\'e Author`);
});

it('does not guess a year or DOI for minimal records', () => {
  const bib = paperBibtex(
    { ...paperSummary, accepted_at: undefined, doi: undefined },
    'https://mathesis.example',
  );
  expect(bib).not.toContain('year =');
  expect(bib).not.toContain('doi =');
});

describe('author names written "Family, Given" or "Given Family"', () => {
  const origin = 'https://mathesis.example';
  const delivered = [{ name: 'Ma, Haobo' }, { name: 'Zhang, Wenlin' }];
  const natural = [{ name: 'Haobo Ma' }, { name: 'Wenlin Zhang' }];
  const others = [
    { name: 'Smith, John, Jr.' },
    { name: 'Research and Development Group' },
    { name: '张文林' },
  ];

  it('cites the canonical display form in plain text for either stored form', () => {
    for (const authors of [delivered, natural]) {
      expect(paperCitation({ ...paperSummary, authors }, origin)).toMatch(
        /^Haobo Ma and Wenlin Zhang\. /,
      );
    }
    expect(paperCitation({ ...paperSummary, authors: others }, origin)).toMatch(
      /^Smith, John, Jr\., Research and Development Group and 张文林\. /,
    );
  });

  it('exports the same valid BibTeX names for either stored form', () => {
    const field = (authors: { name: string }[]) =>
      /^ {2}author = \{(.*)\},$/m.exec(paperBibtex({ ...paperSummary, authors }, origin))?.[1];
    expect(field(delivered)).toBe('{Haobo Ma} and {Wenlin Zhang}');
    expect(field(natural)).toBe(field(delivered));
    // Every other name stays one brace-protected BibTeX name, exactly as stored.
    expect(field(others)).toBe(
      '{Smith, John, Jr.} and {Research and Development Group} and {张文林}',
    );
  });
});
