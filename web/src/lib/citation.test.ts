import { describe, expect, it } from 'vitest';
import { paperSummary } from '../test/fixtures';
import { paperCitation } from './citation';

describe('paperCitation', () => {
  it('includes the DOI and its resolver link', () => {
    const citation = paperCitation(paperSummary, 'https://wishpool.example');
    expect(citation).toContain(
      'author DOI: 10.48550/arXiv.2609.33421 (https://doi.org/10.48550/arXiv.2609.33421).',
    );
    expect(citation).toContain('https://wishpool.example/papers/WP-2026-0001');
  });

  it('omits the DOI when the paper has none', () => {
    expect(
      paperCitation({ ...paperSummary, doi: undefined }, 'https://wishpool.example'),
    ).not.toContain('doi:');
  });

  it('uses the Mathesis DOI as the primary citation and keeps an author DOI distinct', () => {
    const citation = paperCitation(
      {
        ...paperSummary,
        publication_doi: '10.9999/mathesis.opaque',
      },
      'https://wishpool.example',
    );
    expect(citation).toContain('doi:10.9999/mathesis.opaque');
    expect(citation).toContain('author DOI: 10.48550/arXiv.2609.33421');
  });
});
