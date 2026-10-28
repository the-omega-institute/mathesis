import { Gunzip, strFromU8, strToU8, unzipSync, zipSync } from 'fflate';
import type { EvidenceAttachment } from '../api/types';

export type EvidenceInput = Omit<EvidenceAttachment, 'bytes' | 'sha256' | 'path'> & { file: File };
const LIMIT = 64 * 1024 * 1024;
export function safePath(path: string): string {
  if (
    !path ||
    path.length > 512 ||
    /[\\:]/.test(path) ||
    path.includes('\0') ||
    path.split('/').some((p) => !p || p === '.' || p === '..')
  )
    throw new Error(`Unsafe archive path: ${path}`);
  return path;
}
export function unpackSource(bytes: Uint8Array, filename: string): Record<string, Uint8Array> {
  if (bytes.length > 30 * 1024 * 1024) throw new Error('Upload must be at most 30 MB.');
  const files: Record<string, Uint8Array> = Object.create(null) as Record<string, Uint8Array>;
  const seen = new Set<string>();
  let total = 0;
  const add = (path: string, data: Uint8Array) => {
    path = path.replace(/^(\.\/)+/, '');
    if (path.startsWith('__MACOSX/')) return;
    safePath(path);
    if (seen.has(path.toLowerCase())) throw new Error(`Duplicate archive path: ${path}`);
    seen.add(path.toLowerCase());
    total += data.length;
    if (total > LIMIT || seen.size > 2000)
      throw new Error('Archive exceeds 2,000 files or 64 MB expanded.');
    files[path] = data;
  };
  if (bytes[0] === 0x50 && bytes[1] === 0x4b) {
    let declared = 0;
    const archiveNames = new Set<string>();
    const entries = unzipSync(bytes, {
      filter: (entry) => {
        if (entry.name.endsWith('/')) return false;
        safePath(entry.name.replace(/^(\.\/)+/, ''));
        const key = entry.name.toLowerCase();
        if (archiveNames.has(key)) throw new Error(`Duplicate archive path: ${entry.name}`);
        archiveNames.add(key);
        if (archiveNames.size > 2000) throw new Error('Archive exceeds 2,000 files.');
        declared += entry.originalSize;
        if (declared > LIMIT) throw new Error('Archive expands beyond 64 MB.');
        return true;
      },
    });
    for (const [name, data] of Object.entries(entries)) add(name, data);
  } else {
    if (bytes[0] === 0x1f && bytes[1] === 0x8b) {
      const chunks: Uint8Array[] = [];
      let size = 0;
      const gzip = new Gunzip((chunk) => {
        size += chunk.length;
        if (size > LIMIT) throw new Error('Archive expands beyond 64 MB.');
        chunks.push(chunk);
      });
      for (let offset = 0; offset < bytes.length; offset += 16384)
        gzip.push(bytes.subarray(offset, offset + 16384), offset + 16384 >= bytes.length);
      bytes = new Uint8Array(size);
      let offset = 0;
      for (const chunk of chunks) {
        bytes.set(chunk, offset);
        offset += chunk.length;
      }
    }
    if (strFromU8(bytes.subarray(257, 262)) === 'ustar') {
      const text = (a: number, b: number) => strFromU8(bytes.subarray(a, b)).split('\0')[0] ?? '';
      for (let at = 0; at + 512 <= bytes.length && bytes[at] !== 0;) {
        const name = text(at, at + 100);
        const prefix = text(at + 345, at + 500);
        const size = parseInt(text(at + 124, at + 136).trim(), 8);
        const type = bytes[at + 156];
        if (!Number.isSafeInteger(size) || size < 0 || at + 512 + size > bytes.length)
          throw new Error('Invalid tar entry.');
        if (type === 0 || type === 48)
          add(prefix ? `${prefix}/${name}` : name, bytes.slice(at + 512, at + 512 + size));
        else if (type !== 53)
          throw new Error(
            'Use a ZIP or regular tar archive without links or extended headers when adding attachments.',
          );
        at += 512 + Math.ceil(size / 512) * 512;
      }
    } else add(filename.replace(/\.gz$/, ''), bytes);
  }
  return files;
}

export async function withEvidence(source: File, evidence: EvidenceInput[]): Promise<File> {
  if (!evidence.length) return source;
  if (source.size > 30 * 1024 * 1024) throw new Error('Source upload must be at most 30 MB.');
  if (evidence.length > 100) throw new Error('Attach at most 100 evidence files.');
  const files = unpackSource(new Uint8Array(await source.arrayBuffer()), source.name);
  const existing: unknown = files['mathesis-evidence.json']
    ? JSON.parse(strFromU8(files['mathesis-evidence.json']))
    : { schema_version: 1, files: [] };
  if (
    !existing ||
    typeof existing !== 'object' ||
    !('schema_version' in existing) ||
    existing.schema_version !== 1 ||
    !('files' in existing) ||
    !Array.isArray(existing.files)
  )
    throw new Error('Invalid existing evidence manifest.');
  const descriptors: unknown[] = [...existing.files];
  if (
    Object.keys(files).length + evidence.length > 2000 ||
    Object.values(files).reduce((n, bytes) => n + bytes.length, 0) +
      evidence.reduce((n, item) => n + item.file.size, 0) >
      LIMIT
  )
    throw new Error('Combined source and evidence exceeds 2,000 files or 64 MB expanded.');
  for (const { file, ...metadata } of evidence) {
    const path = safePath(`evidence/${file.name}`);
    if (Object.keys(files).some((name) => name.toLowerCase() === path.toLowerCase()))
      throw new Error(`Evidence file already exists: ${path}`);
    if (!metadata.description.trim()) throw new Error(`Describe what ${file.name} supports.`);
    if (
      metadata.source_revision &&
      !/^(?:[a-f\d]{40}|[a-f\d]{64})$/i.test(metadata.source_revision)
    )
      throw new Error('Use a full Git commit hash for evidence source revisions.');
    files[path] = new Uint8Array(await file.arrayBuffer());
    descriptors.push({ ...metadata, claims: metadata.claims.filter(Boolean), path });
  }
  if (
    descriptors.length > 100 ||
    Object.values(files).reduce((n, data) => n + data.length, 0) > LIMIT
  )
    throw new Error('Evidence bundle exceeds 100 attachments or 64 MB expanded.');
  files['mathesis-evidence.json'] = strToU8(
    JSON.stringify({ schema_version: 1, files: descriptors }),
  );
  const bundle = zipSync(files);
  if (bundle.length > 30 * 1024 * 1024)
    throw new Error('Combined source and evidence must be at most 30 MB.');
  return new File([bundle as Uint8Array<ArrayBuffer>], 'mathesis-source.zip', {
    type: 'application/zip',
  });
}

/** Keep a single upload exact; bundle a multi-file manuscript with its paths intact. */
export async function bundleManuscript(files: readonly File[]): Promise<File> {
  if (!files.length) throw new Error('Choose manuscript files.');
  if (files.length === 1) {
    const file = files[0]!;
    const { sourceFileProblem } = await import('./upload');
    const problem = sourceFileProblem(file);
    if (problem) throw new Error(problem);
    return file;
  }
  if (files.length > 2000 || files.reduce((n, f) => n + f.size, 0) > 30 * 1024 * 1024)
    throw new Error('Choose at most 2,000 files totalling 30 MB.');
  if (!files.some((f) => f.name.toLowerCase().endsWith('.tex')))
    throw new Error('Choose one PDF, or include a main .tex file in a multi-file manuscript.');
  const entries: Record<string, Uint8Array> = Object.create(null) as Record<string, Uint8Array>;
  const names = new Set<string>();
  for (const file of files) {
    if (/\.(zip|gz|tgz)$/i.test(file.name)) throw new Error('Upload a source archive on its own.');
    const path = safePath(file.webkitRelativePath || file.name);
    if (names.has(path.toLowerCase())) throw new Error(`Duplicate manuscript path: ${path}`);
    names.add(path.toLowerCase());
    entries[path] = new Uint8Array(await file.arrayBuffer());
  }
  const bytes = zipSync(entries);
  if (bytes.length > 30 * 1024 * 1024)
    throw new Error('The manuscript archive must be at most 30 MB.');
  return new File([bytes as Uint8Array<ArrayBuffer>], 'manuscript.zip', {
    type: 'application/zip',
  });
}
