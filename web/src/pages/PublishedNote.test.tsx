import { act, screen, within, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { setLocale, useLocale } from '../i18n/locale';
import { findProjectNotes, projectNotes, projectNoteVersions } from '../lib/projectNotes';
import { fakeApi, renderWithApp } from '../test/render';
import { PublishedNotePage } from './PublishedNote';
import { paperSummary } from '../test/fixtures';
import userEvent from '@testing-library/user-event';
import { PaperPage, PapersPage } from './Papers';
import { HomePage } from './Home';
import type { CollectedPaper } from '../api/types';

function liveVersion(note = projectNotes[0]!): CollectedPaper {
  return {
    ...note,
    revision: String(Number(note.revision) + 1),
    title: 'The updated published result',
    msc: [],
    pdf: { url: '/api/v1/collected-papers/updated/files/pdf?version=5' },
    source: { url: '/api/v1/collected-papers/updated/files/source?version=5' },
  };
}

it('shows the live successor of a migrated manuscript once in the archive and on its paper page', async () => {
  const old = projectNotes[0]!;
  const live = liveVersion(old);
  const api = fakeApi({
    listCollectedPapers: vi.fn(async () => ({ items: [live], next_before: null })),
    getCollectedPaper: vi.fn(async () => live),
  });
  const archive = renderWithApp(<PapersPage />, api);
  await screen.findByRole('link', { name: live.title });
  expect(screen.queryByRole('link', { name: old.title })).toBeNull();
  expect(screen.getAllByRole('link', { name: 'Read PDF' })).toHaveLength(10);
  archive.unmount();
  renderWithApp(<PaperPage record={old.id} />, api);
  await screen.findByRole('heading', { name: live.title });
  expect(screen.getByRole('link', { name: 'Read PDF' })).toHaveAttribute('href', live.pdf.url);
  expect(api.getPaper).not.toHaveBeenCalled();
});

it('shows a live successor on the home page while a pinned historical URL keeps its version', async () => {
  const old = projectNotes[0]!;
  const live = { ...liveVersion(old), published_on: '2026-12-01T00:00:00Z' };
  const api = fakeApi({
    listCollectedPapers: vi.fn(async () => ({ items: [live], next_before: null })),
  });
  const home = renderWithApp(<HomePage />, api);
  await screen.findByRole('link', { name: live.title });
  expect(screen.queryByRole('link', { name: old.title })).toBeNull();
  home.unmount();
  renderWithApp(<PaperPage record={old.id} version={old.revision} />, api);
  await waitFor(() => expect(api.getCollectedPaper).toHaveBeenCalled());
  expect(screen.getByRole('link', { name: 'Read PDF' })).toHaveAttribute('href', old.pdf.url);
});

describe('published manuscripts', () => {
  it('ships the exact reviewed PDF and source bytes for every version', () => {
    expect(projectNotes).toHaveLength(10);
    for (const note of projectNoteVersions) {
      expect(note.lean!.url).toMatch(/\/blob\/[a-f0-9]{40}\/.+\.lean(?:#L\d+)?$/);
      expect(note.review.basis).toBe('upstream_manuscript_review');
      for (const asset of [note.pdf, note.source]) {
        expect(asset.url).toMatch(
          new RegExp(`^/collections/trureturing/${note.id}/v${note.revision}/[^/]+$`),
        );
        const bytes = readFileSync(resolve('public', asset.url.slice(1)));
        expect(createHash('sha256').update(bytes).digest('hex')).toBe(asset.sha256);
      }
      expect(
        readFileSync(resolve('public', note.pdf.url.slice(1)))
          .subarray(0, 5)
          .toString(),
      ).toBe('%PDF-');
      expect(
        readFileSync(resolve('public', note.source.url.slice(1)))
          .subarray(0, 2)
          .toString(),
      ).toBe('PK');
      // The public snapshot contains scientific metadata, never coordinator receipts.
      expect(JSON.stringify(note)).not.toMatch(
        /task_id|workflow_hash|private_account|signature|bearer|github_token/iu,
      );
    }
  });

  it('keeps original problem sources separate from the note identity and Mathesis decisions', () => {
    const note = projectNotes.find((note) => note.id === 'wang-conjecture326')!;
    const { container } = renderWithApp(<PublishedNotePage id={note.id} />, fakeApi());
    expect(screen.getByRole('heading', { level: 1 })).toHaveTextContent(note.title);
    expect(container).not.toHaveTextContent(/Trureturing|upstream manuscript review|Included from/);
    expect(screen.getByRole('link', { name: 'Read PDF' })).toHaveAttribute('href', note.pdf.url);
    expect(screen.getByRole('link', { name: 'Download source' })).toHaveAttribute(
      'href',
      note.source.url,
    );
    const questions = screen
      .getByText('Sources and related questions', { selector: 'summary' })
      .closest('details')!;
    expect(
      within(questions).getAllByRole('link', { name: 'arXiv:2509.18724v1', hidden: true }),
    ).toHaveLength(2);
    expect(screen.getByRole('link', { name: 'Lean source' })).toHaveAttribute(
      'href',
      note.lean!.url,
    );
    expect(container.querySelector('.badge')).toBeNull();
    expect(screen.getByRole('heading', { name: 'In plain language' })).toBeInTheDocument();
  });

  it('opens sources and the exact-version citation from the paper navigation', async () => {
    const note = projectNotes.find((note) => note.id === 'wang-conjecture326')!;
    renderWithApp(<PublishedNotePage id={note.id} />, fakeApi());
    const navigation = screen.getByRole('navigation', { name: 'On this page' });
    await userEvent.click(
      within(navigation).getByRole('link', { name: 'Sources and related questions' }),
    );
    expect(
      screen.getByText('Sources and related questions', { selector: 'summary' }).closest('details'),
    ).toHaveAttribute('open');
    await userEvent.click(within(navigation).getByRole('link', { name: 'Citation and versions' }));
    expect(document.getElementById('citation')).toHaveAttribute('open');
    expect(document.getElementById('citation')).toHaveTextContent(
      `/papers/${note.id}?version=${note.revision}`,
    );
  });

  it('pins version navigation and rejects unknown versions', () => {
    renderWithApp(<PublishedNotePage id="wang-conjecture326" />, fakeApi());
    expect(
      document.querySelector('a[href="/papers/wang-conjecture326?version=2"]'),
    ).toHaveAttribute('href', '/papers/wang-conjecture326?version=2');
  });

  it('does not fall back to a newer version when the requested version is absent', () => {
    renderWithApp(<PublishedNotePage id="wang-conjecture326" version="999" />, fakeApi());
    expect(screen.queryByRole('link', { name: 'Read PDF' })).toBeNull();
    expect(screen.getByRole('heading', { level: 1 })).toHaveTextContent('Page not found');
  });

  it('finds works by author, title and original source through the shared archive', async () => {
    const matches = findProjectNotes({ q: '2509.18724v1' });
    expect(matches.map((note) => note.id)).toEqual(['wang-conjecture326']);
    expect(findProjectNotes({ author: projectNotes[0]!.authors[0]!.name }).length).toBeGreaterThan(
      0,
    );
    expect(findProjectNotes({ q: 'not-a-matching-scientific-query' })).toEqual([]);
    expect(findProjectNotes({ msc: '11' }).map((note) => note.id)).toContain('b164660-chebfloor');
    expect(findProjectNotes({ msc: '81', q: 'A164660' })).toEqual([]);
    renderWithApp(<PapersPage initialQuery="2509.18724v1" />, fakeApi());
    expect(await screen.findByRole('link', { name: matches[0]!.title })).toHaveAttribute(
      'href',
      '/papers/wang-conjecture326',
    );
    expect(screen.queryByText(/No papers match/)).toBeNull();
  });

  it('shows real research on the home page and follows the global language setting', async () => {
    function LocalizedHome() {
      useLocale();
      return <HomePage />;
    }
    renderWithApp(<LocalizedHome />, fakeApi());
    expect(await screen.findByRole('link', { name: projectNotes[0]!.title })).toBeInTheDocument();
    expect(screen.getAllByRole('link', { name: 'Read PDF' })).toHaveLength(3);
    act(() => setLocale('zh'));
    expect(screen.getByRole('heading', { name: '最新研究' })).toBeInTheDocument();
    expect(screen.queryByText(/Trureturing/)).toBeNull();
    expect(screen.getAllByRole('link', { name: '阅读 PDF' })).toHaveLength(3);
  });

  it('lists all manuscripts in a single bibliography without project branding or review badges', async () => {
    const { container } = renderWithApp(<PapersPage />, fakeApi());
    await waitFor(() => expect(screen.queryByRole('status')).toBeNull());
    expect(screen.getAllByRole('link', { name: 'Read PDF' })).toHaveLength(10);
    expect(screen.getAllByRole('link', { name: 'Source files' })).toHaveLength(10);
    expect(screen.getAllByRole('link', { name: 'Lean source' })).toHaveLength(10);
    expect(container.querySelector('.research-archive-list')?.tagName).toBe('OL');
    for (const [index, note] of projectNotes.entries()) {
      const entry = container.querySelectorAll('.paper-entry')[index]!;
      expect(entry.querySelector('.prose')).toBeNull();
      expect(entry).toHaveTextContent(note.authors[0]!.name);
      expect(entry).toHaveTextContent(`Version ${note.revision}`);
      expect(entry.querySelector('.research-entry-sources')).toBeNull();
    }
    expect(container.querySelector('.badge')).toBeNull();
    expect(container).not.toHaveTextContent(/Trureturing|collection|upstream/);
    expect(container.querySelectorAll('.research-archive-list')).toHaveLength(1);
  });
});

it('opens a published manuscript through the ordinary paper route without calling the review API', () => {
  const api = fakeApi();
  renderWithApp(<PaperPage record="wang-conjecture326" version="2" />, api);
  expect(screen.getByRole('link', { name: 'Read PDF' })).toHaveAttribute(
    'href',
    projectNotes.find((note) => note.id === 'wang-conjecture326')!.pdf.url,
  );
  expect(api.getPaper).not.toHaveBeenCalled();
  expect(screen.getByRole('link', { name: 'Research archive' })).toHaveAttribute('href', '/papers');
  expect(screen.getByText(/2026\. http/)).not.toHaveTextContent('Trureturing');
});

it('combines intake paths by date and does not repeat local publications on later pages', async () => {
  const newer = { ...paperSummary, accepted_at: '2026-12-01T00:00:00Z' };
  const older = { ...paperSummary, record: 'older', title: 'An older paper' };
  const listPapers = vi.fn(async (query) =>
    query?.before
      ? { items: [older], next_before: null }
      : { items: [newer], next_before: 'next-cursor' },
  );
  const { container } = renderWithApp(<PapersPage />, fakeApi({ listPapers }));
  await screen.findByRole('link', { name: newer.title });
  expect(container.querySelectorAll('.research-archive-list > li')).toHaveLength(11);
  expect(container.querySelector('.research-archive-list > li')).toHaveTextContent(newer.title);
  await userEvent.click(screen.getByRole('button', { name: 'Older →' }));
  await screen.findByRole('link', { name: older.title });
  expect(container.querySelectorAll('.research-archive-list > li')).toHaveLength(1);
  expect(listPapers).toHaveBeenLastCalledWith(
    expect.objectContaining({ before: 'next-cursor', limit: 25 }),
    expect.anything(),
  );
  await userEvent.click(screen.getByRole('button', { name: '← Newer' }));
  await screen.findByRole('link', { name: newer.title });
  expect(container.querySelectorAll('.research-archive-list > li')).toHaveLength(11);
});

it('preserves a live exact-version DOI through archive, paper citation and reference-manager metadata', async () => {
  const old = projectNotes[0]!;
  const live = {
    ...liveVersion(old),
    archive: { doi: '10.5281/zenodo.23957560', concept_doi: '10.5281/zenodo.23957559' },
  };
  const api = fakeApi({
    listCollectedPapers: vi.fn(async () => ({ items: [live], next_before: null })),
    getCollectedPaper: vi.fn(async () => live),
  });
  const archive = renderWithApp(<PapersPage initialQuery={live.archive.doi} />, api);
  expect(await screen.findByRole('link', { name: 'DOI' })).toHaveAttribute(
    'href',
    'https://doi.org/' + live.archive.doi,
  );
  archive.unmount();
  const paper = renderWithApp(<PaperPage record={live.id} />, api);
  expect(await screen.findByRole('link', { name: live.archive.doi })).toHaveAttribute(
    'href',
    'https://doi.org/' + live.archive.doi,
  );
  expect(document.getElementById('citation')).toHaveTextContent(
    'https://doi.org/' + live.archive.doi,
  );
  expect(document.getElementById('citation')).not.toHaveTextContent(live.archive.concept_doi);
  expect(document.querySelector('meta[name="citation_doi"]')).toHaveAttribute(
    'content',
    live.archive.doi,
  );
  expect(document.querySelector('meta[name="citation_pdf_url"]')).toHaveAttribute(
    'content',
    new URL(live.pdf.url, window.location.origin).href,
  );
  expect(document.querySelector('link[rel="canonical"]')).toHaveAttribute(
    'href',
    `${window.location.origin}/papers/${live.id}?version=${live.revision}`,
  );
  expect(document.querySelector('meta[name="citation_journal_title"]')).toBeNull();
  paper.unmount();
  renderWithApp(<PublishedNotePage id={old.id} note={old} />, fakeApi());
  expect(document.querySelector('meta[name="citation_doi"]')).toBeNull();
  expect(document.getElementById('citation')).toHaveTextContent(
    `/papers/${old.id}?version=${old.revision}`,
  );
});
