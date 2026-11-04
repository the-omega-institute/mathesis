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

describe('published manuscripts', () => {
  it('ships the exact reviewed PDF and source bytes for every version', () => {
    expect(projectNotes).toHaveLength(10);
    for (const note of projectNoteVersions) {
      expect(note.lean.url).toMatch(/\/blob\/[a-f0-9]{40}\/.+\.lean(?:#L\d+)?$/);
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
      .getByRole('heading', { name: 'Original questions and sources' })
      .closest('section')!;
    expect(within(questions).getAllByRole('link', { name: 'arXiv:2509.18724v1' })).toHaveLength(2);
    expect(screen.getByRole('link', { name: 'Lean source' })).toHaveAttribute(
      'href',
      note.lean.url,
    );
    expect(container.querySelector('.badge')).toBeNull();
    expect(screen.queryByText(/Lean verified|Mathesis accepted|In plain language/)).toBeNull();
  });

  it('pins version navigation and rejects unknown versions', () => {
    renderWithApp(<PublishedNotePage id="wang-conjecture326" />, fakeApi());
    expect(screen.getByRole('link', { name: 'Version 2' })).toHaveAttribute(
      'href',
      '/papers/wang-conjecture326?version=2',
    );
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
    expect(findProjectNotes({ msc: '11' })).toEqual([]);
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
    expect(screen.getAllByRole('link', { name: 'Source' })).toHaveLength(10);
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
