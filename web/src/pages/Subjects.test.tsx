import { act, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { App } from '../App';
import { setLocale } from '../i18n/locale';
import { projectNotes } from '../lib/projectNotes';
import { paperSummary } from '../test/fixtures';
import { fakeApi, renderWithApp } from '../test/render';
import { SubjectPage } from './Subjects';

afterEach(() => window.history.replaceState(null, '', '/'));

describe('subject browsing', () => {
  it('opens a subject from home as a publication list, then switches through the directory', async () => {
    window.history.replaceState(null, '', '/');
    const listPapers = vi.fn(async () => ({ items: [], next_before: null }));
    renderWithApp(<App />, fakeApi({ listPapers }));
    const user = userEvent.setup();
    await user.click(screen.getByRole('link', { name: 'Number theory' }));
    expect(await screen.findByRole('heading', { level: 1, name: 'Number theory' })).toBeVisible();
    expect(window.location.pathname).toBe('/subjects/11');
    expect(window.location.search).toBe('');
    expect(listPapers).toHaveBeenLastCalledWith(
      { msc: '11', before: null, limit: 25 },
      expect.anything(),
    );
    expect(within(screen.getByRole('main')).queryByRole('search')).toBeNull();
    const numberNote = projectNotes.find((note) => note.id === 'b164660-chebfloor')!;
    expect(screen.getByRole('link', { name: numberNote.title })).toBeVisible();

    await user.click(screen.getByRole('link', { name: 'All subjects' }));
    expect(screen.getByRole('heading', { level: 1, name: 'Browse by subject' })).toBeVisible();
    await user.click(screen.getByRole('link', { name: 'Quantum theory' }));
    expect(await screen.findByRole('heading', { level: 1, name: 'Quantum theory' })).toBeVisible();
    expect(screen.queryByRole('link', { name: numberNote.title })).toBeNull();
    expect(screen.getAllByRole('link', { name: 'Read PDF' })).toHaveLength(3);
    expect(listPapers).toHaveBeenLastCalledWith(
      { msc: '81', before: null, limit: 25 },
      expect.anything(),
    );
    act(() => setLocale('zh'));
    expect(screen.getByRole('heading', { level: 1, name: '量子理论' })).toBeVisible();
    expect(document.title).toBe('量子理论 · Mathesis');
  });

  it('keeps the subject while paging and includes local manuscripts only on the first page', async () => {
    const listPapers = vi.fn(async (query) => ({
      items: [{ ...paperSummary, title: query?.before ? 'An older paper' : 'A recent paper' }],
      next_before: query?.before ? null : 'next',
    }));
    renderWithApp(<SubjectPage msc="11" />, fakeApi({ listPapers }));
    await screen.findByRole('link', { name: 'A recent paper' });
    const note = projectNotes.find((note) => note.id === 'b164660-chebfloor')!;
    expect(screen.getByRole('link', { name: note.title })).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Older →' }));
    await screen.findByRole('link', { name: 'An older paper' });
    expect(screen.queryByRole('link', { name: note.title })).toBeNull();
    expect(listPapers).toHaveBeenLastCalledWith(
      { msc: '11', before: 'next', limit: 25 },
      expect.anything(),
    );
    await userEvent.click(screen.getByRole('button', { name: '← Newer' }));
    await screen.findByRole('link', { name: 'A recent paper' });
    expect(screen.getByRole('link', { name: note.title })).toBeVisible();
  });

  it('gives an empty subject a clear message and a way back to the directory', async () => {
    renderWithApp(<SubjectPage msc="86" />, fakeApi());
    expect(
      await screen.findByText('No papers have been listed in this subject yet.'),
    ).toBeVisible();
    expect(screen.getByRole('link', { name: 'All subjects' })).toHaveAttribute('href', '/subjects');
  });

  it('does not query the archive for an unknown subject', async () => {
    const api = fakeApi();
    renderWithApp(<SubjectPage msc="99" />, api);
    await waitFor(() =>
      expect(screen.getByRole('heading', { name: 'Page not found' })).toBeVisible(),
    );
    expect(api.listPapers).not.toHaveBeenCalled();
  });
});
