import { describe, expect, it, vi } from 'vitest';
import { readLeanSourceLink } from './leanSourceLink';

const revision = 'a'.repeat(40);
const link = `https://github.com/example/research/blob/${revision}/Proofs/Example.lean`;

describe('linked Lean sources', () => {
  it('imports exact pinned bytes without forwarding credentials or following redirects', async () => {
    const source = 'import Mathlib\ntheorem example_result : True := True.intro\n';
    const fetchSource = vi.fn<typeof fetch>(async () => new Response(source));
    expect(await readLeanSourceLink(link, fetchSource)).toEqual({
      url: link,
      revision,
      filename: 'Example.lean',
      moduleName: 'Proofs.Example',
      source,
    });
    expect(fetchSource).toHaveBeenCalledWith(
      `https://raw.githubusercontent.com/example/research/${revision}/Proofs/Example.lean`,
      expect.objectContaining({
        credentials: 'omit',
        redirect: 'error',
        referrerPolicy: 'no-referrer',
      }),
    );
    const raw = `https://raw.githubusercontent.com/example/research/${revision}/Proofs/Example.lean`;
    expect((await readLeanSourceLink(raw, fetchSource)).url).toBe(link);
  });
  it.each([
    'https://github.com/example/research/blob/main/Proof.lean',
    'https://github.com/example/research',
    'https://example.org/Proof.lean',
    `https://user:secret@github.com/example/research/blob/${revision}/Proof.lean`,
    link + '?token=secret',
    link.replace('.lean', '.html'),
  ])('refuses a mutable or unsupported location without fetching it: %s', async (url) => {
    const fetchSource = vi.fn<typeof fetch>();
    await expect(readLeanSourceLink(url, fetchSource)).rejects.toThrow();
    expect(fetchSource).not.toHaveBeenCalled();
  });
  it('bounds streaming downloads even without a content-length header', async () => {
    const fetchSource = vi.fn<typeof fetch>(async () => new Response('x'.repeat(1_048_577)));
    await expect(readLeanSourceLink(link, fetchSource)).rejects.toThrow('at most 1 MB');
  });
  it.each(['', '<html>not a source</html>', 'bad\0source'])(
    'rejects unusable source bytes',
    async (source) => {
      const fetchSource = vi.fn<typeof fetch>(async () => new Response(source));
      await expect(readLeanSourceLink(link, fetchSource)).rejects.toThrow('Lean source file');
    },
  );
});
