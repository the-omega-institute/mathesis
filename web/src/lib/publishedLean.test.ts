import { expect, it } from 'vitest';
import { publishedLeanSource } from './publishedLean';

it('requires the exact pinned Lean source and never accepts a floating branch, mismatched revision or unrelated URL', () => {
  const revision = 'a'.repeat(40);
  const source = {
    url: `https://github.com/example/research/blob/${revision}/Proof.lean#L20`,
    revision,
  };
  expect(publishedLeanSource({ lean: source })).toBe(source.url);
  for (const url of [
    source.url.replace(revision, 'main'),
    source.url.replace(revision, 'b'.repeat(40)),
    source.url.replace('github.com', 'example.org'),
    'javascript:alert(1)',
    source.url.replace('.lean', '.md'),
  ])
    expect(publishedLeanSource({ lean: { ...source, url } })).toBeUndefined();
  expect(publishedLeanSource({})).toBeUndefined();
});
