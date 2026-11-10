import { screen } from '@testing-library/react';
import { expect, it } from 'vitest';
import { fakeApi, renderWithApp } from '../test/render';
import { AuthorLine } from './AuthorLine';
import { CitationMetadata } from './PaperMetadata';

const authors = [
  { name: 'Ma, Haobo', orcid: '0000-0002-1825-0097' },
  { name: 'Haobo Ma' },
  { name: 'Smith, John, Jr.' },
  { name: '张文林' },
];

it('shows one written form and one archive link for either stored form of a name', () => {
  renderWithApp(<AuthorLine authors={authors} />, fakeApi());
  const links = screen.getAllByRole('link').filter((link) => link.className === 'author-name');
  expect(links.map((link) => link.textContent)).toEqual([
    'Haobo Ma',
    'Haobo Ma',
    'Smith, John, Jr.',
    '张文林',
  ]);
  expect(links[0]).toHaveAttribute('href', '/papers?author=Haobo+Ma');
  expect(links[1]).toHaveAttribute('href', '/papers?author=Haobo+Ma');
  // Names outside the single-comma form keep their stored text and query.
  expect(links[2]).toHaveAttribute('href', '/papers?author=Smith%2C+John%2C+Jr.');
  expect(screen.getByLabelText('ORCID iD of Haobo Ma')).toBeInTheDocument();
});

it('gives reference managers the same names as the reader', () => {
  renderWithApp(
    <CitationMetadata title="A note" authors={authors} canonicalUrl="https://mathesis.example/" />,
    fakeApi(),
  );
  const names = [...document.querySelectorAll('meta[name="citation_author"]')].map((meta) =>
    meta.getAttribute('content'),
  );
  expect(names).toEqual(['Haobo Ma', 'Haobo Ma', 'Smith, John, Jr.', '张文林']);
});
