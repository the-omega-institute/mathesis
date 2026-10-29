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
