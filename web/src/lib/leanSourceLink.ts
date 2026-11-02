export interface LinkedLeanSource {
  url: string;
  revision: string;
  filename: string;
  source: string;
}

/** Import public, immutable GitHub source; never forward Mathesis credentials. */
export async function readLeanSourceLink(
  input: string,
  fetchSource: typeof fetch = fetch,
): Promise<LinkedLeanSource> {
  if (input.length > 2000) throw new Error('Use a link of at most 2,000 characters.');
  const url = new URL(input.trim());
  const parts = url.pathname.slice(1).split('/');
  if (
    url.protocol !== 'https:' ||
    url.username ||
    url.password ||
    url.port ||
    url.search ||
    url.hash ||
    !['github.com', 'raw.githubusercontent.com'].includes(url.hostname)
  )
    throw new Error('Use a public GitHub .lean file link at a full commit revision.');
  const [owner, repo] = parts;
  if (url.hostname === 'github.com') {
    if (parts[2] !== 'blob') throw new Error('Open the .lean file and copy its permanent link.');
    parts.splice(2, 1);
  }
  const revision = parts[2] ?? '';
  const path = parts.slice(3);
  if (
    !owner ||
    !repo ||
    !/^(?:[a-fA-F0-9]{40}|[a-fA-F0-9]{64})$/.test(revision) ||
    !path.length ||
    path.some((p) => !p || !/^[\w.-]+$/.test(p) || p === '.' || p === '..') ||
    !path.at(-1)?.endsWith('.lean')
  )
    throw new Error('Use a .lean file at a full Git commit, rather than a branch or repository.');
  const raw = `https://raw.githubusercontent.com/${parts.join('/')}`;
  const canonical = `https://github.com/${owner}/${repo}/blob/${revision}/${path.join('/')}`;
  const response = await fetchSource(raw, {
    credentials: 'omit',
    referrerPolicy: 'no-referrer',
    redirect: 'error',
    signal: AbortSignal.timeout(30_000),
  });
  if (!response.ok || !response.body)
    throw new Error('The source could not be read. Check that the pinned file is public.');
  const limit = 1_048_576;
  if (Number(response.headers.get('content-length')) > limit) {
    await response.body.cancel();
    throw new Error('Linked Lean source must be at most 1 MB.');
  }
  const reader = response.body.getReader();
  const chunks: Uint8Array[] = [];
  let bytes = 0;
  try {
    for (;;) {
      const { done, value } = await reader.read();
      if (done) break;
      bytes += value.byteLength;
      if (bytes > limit) throw new Error('Linked Lean source must be at most 1 MB.');
      chunks.push(value);
    }
  } finally {
    await reader.cancel();
    reader.releaseLock();
  }
  const content = new Uint8Array(bytes);
  let offset = 0;
  for (const chunk of chunks) {
    content.set(chunk, offset);
    offset += chunk.byteLength;
  }
  const source = new TextDecoder('utf-8', { fatal: true, ignoreBOM: true }).decode(content);
  if (!source.trim() || source.includes('\0') || /^\s*<!?(?:doctype|html)/i.test(source))
    throw new Error('The link did not return a Lean source file.');
  return { url: canonical, revision, filename: path.at(-1)!, source };
}
