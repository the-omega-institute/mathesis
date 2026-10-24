import { screen } from '@testing-library/react';
import { expect, it } from 'vitest';
import { fakeApi, renderWithApp } from '../test/render';
import { PolicyPage } from './Policy';

it('opens the review policy with a plain-words summary before the formal definitions', async () => {
  renderWithApp(<PolicyPage />, fakeApi({}));
  const plain = await screen.findByRole('heading', { name: 'In plain words' });
  const formal = await screen.findByRole('heading', {
    name: 'Escape analysis: does the proof add new content?',
  });
  expect(plain.compareDocumentPosition(formal) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  const items = screen.getAllByRole('listitem').map((item) => item.textContent ?? '');
  expect(items.some((text) => text.includes('confirm the list of statements'))).toBe(true);
  expect(items.some((text) => text.includes('Reviewed by GPT Pro and audited by Codex'))).toBe(
    true,
  );
  expect(items.some((text) => text.includes('about 45 minutes'))).toBe(true);
});
