import { describe, expect, it } from 'vitest';
import { parseRoute, routePath, type Route } from './routes';

const ROUTES: Route[] = [
  { kind: 'home' },
  { kind: 'papers' },
  { kind: 'papers', q: 'Ada Example' },
  { kind: 'submission', id: 'sub-1', staff: true },
  { kind: 'formalizations' },
  { kind: 'submit', intake: 'lean' },
  { kind: 'submit', intake: 'review' },
  { kind: 'submit', intake: 'lean', record: 'WP-2026-0001', claim: 'C1' },
  { kind: 'paper', record: 'WP-2026-0001' },
  { kind: 'policy' },
  { kind: 'submit' },
  { kind: 'submissions' },
  { kind: 'queue' },
  { kind: 'submission', id: 'sub-1' },
  { kind: 'people' },
  { kind: 'authors' },
  { kind: 'author', name: 'Ada Lovelace' },
  { kind: 'contribute' },
  { kind: 'tasks' },
  { kind: 'tasks', submission: 'sub 1' },
  { kind: 'tasks', submission: 'sub-1', taskKind: 'literature_check' },
  { kind: 'task', id: 'task-1' },
  { kind: 'contributors' },
];

describe('routes', () => {
  it.each(ROUTES)('round-trips %o', (route) => {
    const path = routePath(route);
    const [pathname, search = ''] = path.split('?');
    expect(parseRoute(pathname ?? '', search ? `?${search}` : '')).toEqual(route);
  });

  it('tolerates a trailing slash', () => {
    expect(parseRoute('/papers/')).toEqual({ kind: 'papers' });
  });

  it('ignores an unknown task kind in the query', () => {
    expect(parseRoute('/tasks', '?kind=wish')).toEqual({ kind: 'tasks' });
  });

  it.each([
    '/nope',
    '/papers/a/b',
    '/papers/%',
    '/submissions/..',
    '/wishes',
    '/ledger',
    '/records',
  ])('rejects %s', (p) => {
    expect(parseRoute(p)).toEqual({ kind: 'not_found', path: p });
  });
});

it('round-trips a shareable academic search and a permanent result link', () => {
  const route = {
    kind: 'papers' as const,
    q: 'prime gaps',
    author: 'A. Author',
    msc: '11',
    work: 'paper' as const,
  };
  const url = new URL(routePath(route), 'https://mathesis.example');
  expect(parseRoute(url.pathname, url.search)).toEqual(route);
  const result = { kind: 'result' as const, record: 'WP-2026-0001', claim: 'C1' };
  expect(parseRoute(routePath(result))).toEqual(result);
});
