import { render, screen, within } from '@testing-library/react';
import { afterEach, expect, it } from 'vitest';
import type { ProofBasis } from '../api/types';
import { setLocale } from '../i18n/locale';
import { ProofBasisNote } from './ProofBasis';

const revision = 'b'.repeat(40);
const library: ProofBasis = {
  kind: 'library_declarations',
  libraries: [{ name: 'trureturing', revision, url: 'https://github.com/example/library' }],
  declarations: [
    {
      library: 'trureturing',
      module: 'D5.S3.Derangements',
      name: 'D5.S3.Derangements.result',
      source: `https://github.com/example/library/blob/${revision}/D5/S3/Derangements.lean`,
    },
    { library: 'trureturing', module: 'D5.S3.Terminal', name: 'D5.S3.Terminal.T' },
  ],
  total: 2,
  complete: true,
};
afterEach(() => setLocale('en'));

it('names the library, its pinned revision and each referenced declaration', () => {
  render(<ProofBasisNote basis={library} />);
  expect(screen.getByText('Proved from library declarations')).toBeVisible();
  const note = screen.getByRole('group');
  expect(within(note).getByText(/follows from these library declarations/)).toBeInTheDocument();
  expect(within(note).getByText(`trureturing at revision ${revision}`)).toBeInTheDocument();
  expect(within(note).getByText('D5.S3.Derangements.result')).toBeInTheDocument();
  expect(within(note).getByText('D5.S3.Terminal.T')).toBeInTheDocument();
  // A link exists only where the receipt gave a source address.
  const links = within(note).getAllByRole('link');
  expect(links).toHaveLength(1);
  expect(links[0]).toHaveAttribute(
    'href',
    `https://github.com/example/library/blob/${revision}/D5/S3/Derangements.lean`,
  );
  expect(links[0]).toHaveAccessibleName('Library source of D5.S3.Derangements.result');
  expect(note.textContent).not.toMatch(/equivalent/i);
  expect(within(note).queryByText(/Showing/)).toBeNull();
  expect(within(note).queryByText(/stopped at its bound/)).toBeNull();
});

it('states the limits of a truncated or incomplete report', () => {
  render(<ProofBasisNote basis={{ ...library, total: 70, complete: false }} />);
  expect(screen.getByText('Showing 2 of 70 referenced library declarations.')).toBeInTheDocument();
  expect(
    screen.getByText(
      'The inspection stopped at its bound, so further library declarations may be referenced.',
    ),
  ).toBeInTheDocument();
});

it('never renders an unsafe source address as a link', () => {
  render(
    <ProofBasisNote
      basis={{
        ...library,
        declarations: [{ ...library.declarations[0]!, source: 'javascript:alert(1)' }],
        total: 1,
      }}
    />,
  );
  expect(screen.queryByRole('link')).toBeNull();
  expect(screen.getByText('D5.S3.Derangements.result')).toBeInTheDocument();
});

it('labels a verified proof without library declarations as new Lean, and nothing when unverified', () => {
  const { rerender, container } = render(<ProofBasisNote basis={{ kind: 'new_lean' }} />);
  expect(screen.getByText('New Lean proof')).toBeVisible();
  expect(
    screen.getByText('The checked proof references no declaration of a prepared library.'),
  ).toBeInTheDocument();
  rerender(<ProofBasisNote basis={{ kind: 'new_lean' }} verdict="disproved" />);
  expect(screen.getByText('New Lean disproof')).toBeVisible();
  rerender(<ProofBasisNote basis={library} verdict="disproved" />);
  expect(screen.getByText('Disproved from library declarations')).toBeVisible();
  rerender(<ProofBasisNote basis={null} />);
  expect(container).toBeEmptyDOMElement();
  rerender(<ProofBasisNote />);
  expect(container).toBeEmptyDOMElement();
});

it('keeps a list entry to the label and the pinned library', () => {
  const { rerender } = render(<ProofBasisNote basis={library} compact />);
  expect(screen.getByText('Proved from library declarations')).toBeVisible();
  expect(screen.getByText(`trureturing at revision ${revision}`)).toBeVisible();
  expect(screen.queryByText('D5.S3.Derangements.result')).toBeNull();
  expect(screen.queryByRole('link')).toBeNull();
  rerender(<ProofBasisNote basis={{ kind: 'new_lean' }} compact />);
  expect(screen.getByText('New Lean proof')).toBeVisible();
  expect(
    screen.queryByText('The checked proof references no declaration of a prepared library.'),
  ).toBeNull();
});

it('has Simplified Chinese labels', () => {
  setLocale('zh');
  const { rerender } = render(<ProofBasisNote basis={library} />);
  expect(screen.getByText('由库中已有声明证明')).toBeVisible();
  expect(screen.getByText(`trureturing，修订版本 ${revision}`)).toBeInTheDocument();
  rerender(<ProofBasisNote basis={{ kind: 'new_lean' }} />);
  expect(screen.getByText('新写的 Lean 证明')).toBeVisible();
});
