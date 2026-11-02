import { act, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { App } from '../App';
import { author } from '../test/fixtures';
import { fakeApi, renderWithApp } from '../test/render';
import { formatDate } from '../lib/format';
import { getLocale, setLocale } from './locale';

beforeEach(() => {
  window.history.replaceState(null, '', '/');
  // Node 26 exposes an unavailable native localStorage in Vitest. Supply a
  // browser-shaped store for the preference tests; production uses the browser.
  const entries = new Map<string, string>();
  vi.stubGlobal('localStorage', {
    get length() {
      return entries.size;
    },
    getItem: (key: string) => entries.get(key) ?? null,
    setItem: (key: string, value: string) => {
      entries.set(key, value);
    },
    removeItem: (key: string) => {
      entries.delete(key);
    },
    clear: () => entries.clear(),
    key: (index: number) => [...entries.keys()][index] ?? null,
  } satisfies Storage);
});
afterEach(() => vi.unstubAllGlobals());

it('uses one persisted language for navigation, pages and dates without losing the search query', async () => {
  const user = userEvent.setup();
  const api = fakeApi();
  renderWithApp(<App />, api);
  const search = screen.getByRole('searchbox', { name: 'Search all research' });
  await user.type(search, 'finite groups');
  await user.click(screen.getByRole('button', { name: '中文' }));
  expect(document.documentElement).toHaveAttribute('lang', 'zh-Hans');
  expect(window.localStorage.getItem('mathesis.language')).toBe('zh');
  expect(screen.getByRole('heading', { level: 1 })).toHaveTextContent('让数学成果得到审阅');
  expect(screen.getByRole('searchbox', { name: '搜索所有研究' })).toHaveValue('finite groups');
  expect(formatDate('2026-11-03T00:00:00Z')).toBe('2026年11月3日');
  await user.click(
    within(screen.getByRole('navigation', { name: '主导航' })).getByRole('link', { name: '猜想' }),
  );
  expect(screen.getByRole('heading', { name: '猜想', level: 1 })).toBeVisible();
  expect(getLocale()).toBe('zh');
  await user.click(screen.getByRole('button', { name: 'English' }));
  expect(document.documentElement).toHaveAttribute('lang', 'en');
  expect(screen.getByRole('heading', { name: 'Conjectures', level: 1 })).toBeVisible();
  expect(formatDate('2026-11-03T00:00:00Z')).toBe('3 November 2026');
});

it.each(['enter', 'button'])('submits the integrated header search using %s', async (mode) => {
  const user = userEvent.setup();
  const api = fakeApi();
  renderWithApp(<App />, api);
  const form = screen.getByRole('search', { name: 'Search the archive' });
  const field = within(form).getByRole('searchbox');
  await user.type(field, '  A & B  ');
  if (mode === 'enter') await user.keyboard('{Enter}');
  else await user.click(within(form).getByRole('button', { name: 'Search' }));
  expect(window.location.pathname).toBe('/papers');
  expect(new URLSearchParams(window.location.search).get('q')).toBe('A & B');
  expect(api.listPapers).toHaveBeenCalledWith(
    expect.objectContaining({ q: 'A & B' }),
    expect.anything(),
  );
});

it('keeps the current submission step, entered mathematics and author profiles when language changes', async () => {
  window.history.replaceState(null, '', '/submit?type=conjecture');
  const user = userEvent.setup();
  const api = fakeApi({}, author);
  renderWithApp(<App />, api);
  const title = await screen.findByLabelText('Title');
  await user.type(title, 'A theorem titled Papers');
  await user.type(
    screen.getByLabelText('Conjecture statement'),
    'Every test preserves its hypothesis.',
  );
  await user.click(screen.getByRole('button', { name: 'Continue' }));
  await user.type(screen.getByLabelText('Name'), 'Original Author');
  await user.click(screen.getByText('Academic profiles (optional)'));
  await user.type(screen.getByLabelText('Website or other link'), 'https://example.edu/original');
  await user.click(screen.getByRole('button', { name: '中文' }));
  expect(screen.getByLabelText('姓名')).toHaveValue('Original Author');
  expect(screen.getByLabelText('个人网站或其他链接')).toHaveValue('https://example.edu/original');
  await user.click(screen.getByRole('button', { name: '上一步' }));
  expect(screen.getByLabelText('标题')).toHaveValue('A theorem titled Papers');
  expect(screen.getByLabelText('猜想表述')).toHaveValue('Every test preserves its hypothesis.');
  expect(api.createSubmission).not.toHaveBeenCalled();
});

it('syncs another tab’s preference, and still works when browser storage is unavailable', () => {
  renderWithApp(<App />, fakeApi());
  act(() => {
    window.localStorage.setItem('mathesis.language', 'zh');
    window.dispatchEvent(new StorageEvent('storage', { key: 'mathesis.language', newValue: 'zh' }));
  });
  expect(screen.getByRole('button', { name: '中文' })).toHaveAttribute('aria-pressed', 'true');
  const failingStorage = vi.spyOn(window.localStorage, 'setItem').mockImplementation(() => {
    throw new Error('Storage unavailable');
  });
  act(() => setLocale('en'));
  expect(getLocale()).toBe('en');
  expect(screen.getByRole('heading', { level: 1 })).toHaveTextContent('Mathematics, reviewed');
  failingStorage.mockRestore();
});
