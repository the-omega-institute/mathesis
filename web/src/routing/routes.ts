import type { TaskKind } from '../api/types';

export type Route =
  | { kind: 'home' }
  | { kind: 'bindings' }
  | { kind: 'feedback'; id: string; key: string }
  | { kind: 'binding_profile'; person: string }
  | { kind: 'binding'; id: string; key: string }
  | { kind: 'papers' }
  | { kind: 'conjectures' }
  | { kind: 'formalizations' }
  | { kind: 'conjecture'; record: string; claim: string }
  | { kind: 'leaderboard' }
  | { kind: 'supporters' }
  | { kind: 'statement'; record: string; claim: string }
  | { kind: 'entrant'; id: string }
  | { kind: 'paper'; record: string }
  | { kind: 'policy' }
  | { kind: 'submit'; intake?: 'lean' | 'review'; record?: string; claim?: string }
  | { kind: 'submissions' }
  | { kind: 'queue' }
  | { kind: 'submission'; id: string }
  | { kind: 'staff_submission'; id: string }
  | { kind: 'people' }
  | { kind: 'authors' }
  | { kind: 'author'; name: string }
  | { kind: 'contribute' }
  | { kind: 'tasks'; submission?: string; taskKind?: TaskKind }
  | { kind: 'task'; id: string }
  | { kind: 'contributors' }
  | { kind: 'not_found'; path: string };

const TASK_KIND_VALUES: readonly string[] = ['judge_escape', 'literature_check', 'probe'];

function decodeSegment(value: string): string | null {
  if (value === '') return null;
  try {
    const decoded = decodeURIComponent(value);
    return decoded === '' || decoded === '.' || decoded === '..' || decoded.includes('/')
      ? null
      : decoded;
  } catch {
    return null;
  }
}

/** Parse a location (`pathname` plus optional `search`) into a route. */
export function parseRoute(pathname: string, search = ''): Route {
  const path = pathname.length > 1 ? pathname.replace(/\/+$/, '') : pathname;
  if (path === '/' || path === '') return { kind: 'home' };
  const notFound: Route = { kind: 'not_found', path: pathname };
  if (!path.startsWith('/')) return notFound;
  const segments = path.slice(1).split('/').map(decodeSegment);
  if (segments.some((s) => s === null)) return notFound;
  const [first, second] = segments as string[];

  if (segments.length === 1) {
    switch (first) {
      case 'bindings':
        return { kind: 'bindings' };
      case 'supporters':
        return { kind: 'supporters' };
      case 'leaderboard':
        return { kind: 'leaderboard' };
      case 'conjectures':
        return { kind: 'conjectures' };
      case 'papers':
        return { kind: 'papers' };
      case 'policy':
        return { kind: 'policy' };
      case 'formalizations':
        return { kind: 'formalizations' };
      case 'submit': {
        const params = new URLSearchParams(search);
        return {
          kind: 'submit',
          ...(params.get('type') === 'lean'
            ? { intake: 'lean' as const }
            : params.get('type') === 'review'
              ? { intake: 'review' as const }
              : {}),
          ...(params.get('record') && params.get('claim')
            ? {
                intake: 'lean' as const,
                record: params.get('record')!,
                claim: params.get('claim')!,
              }
            : {}),
        };
      }
      case 'submissions':
        return { kind: 'submissions' };
      case 'queue':
        return { kind: 'queue' };
      case 'people':
        return { kind: 'people' };
      case 'authors':
        return { kind: 'authors' };
      case 'contribute':
        return { kind: 'contribute' };
      case 'tasks': {
        const params = new URLSearchParams(search);
        const route: Route = { kind: 'tasks' };
        const submission = params.get('submission');
        const kind = params.get('kind');
        if (submission) route.submission = submission;
        if (kind && TASK_KIND_VALUES.includes(kind)) route.taskKind = kind as TaskKind;
        return route;
      }
      case 'contributors':
        return { kind: 'contributors' };
    }
    return notFound;
  }
  if (segments.length === 4 && first === 'papers' && segments[2] === 'statements')
    return { kind: 'statement', record: second as string, claim: segments[3] as string };
  if (segments.length === 3 && first === 'feedback')
    return { kind: 'feedback', id: second as string, key: segments[2] as string };
  if (segments.length === 3 && first === 'bindings')
    return { kind: 'binding', id: second as string, key: segments[2] as string };
  if (segments.length === 3 && first === 'conjectures')
    return { kind: 'conjecture', record: second as string, claim: segments[2] as string };
  if (segments.length === 3 && first === 'staff' && second === 'submissions')
    return { kind: 'staff_submission', id: segments[2] as string };
  if (segments.length === 2 && second !== undefined) {
    switch (first) {
      case 'binding-contributors':
        return { kind: 'binding_profile', person: second };
      case 'authors':
        return { kind: 'author', name: second };
      case 'entrants':
        return { kind: 'entrant', id: second };
      case 'papers':
        return { kind: 'paper', record: second };
      case 'submissions':
        return { kind: 'submission', id: second };
      case 'tasks':
        return { kind: 'task', id: second };
    }
  }
  return notFound;
}

const seg = encodeURIComponent;

export function routePath(route: Route): string {
  switch (route.kind) {
    case 'feedback':
      return `/feedback/${seg(route.id)}/${seg(route.key)}`;
    case 'binding_profile':
      return `/binding-contributors/${seg(route.person)}`;
    case 'bindings':
      return '/bindings';
    case 'binding':
      return `/bindings/${seg(route.id)}/${seg(route.key)}`;
    case 'supporters':
      return '/supporters';
    case 'statement':
      return `/papers/${seg(route.record)}/statements/${seg(route.claim)}`;
    case 'home':
      return '/';
    case 'leaderboard':
      return '/leaderboard';
    case 'entrant':
      return `/entrants/${seg(route.id)}`;
    case 'conjecture':
      return `/conjectures/${seg(route.record)}/${seg(route.claim)}`;
    case 'conjectures':
      return '/conjectures';
    case 'papers':
      return '/papers';
    case 'paper':
      return `/papers/${seg(route.record)}`;
    case 'policy':
      return '/policy';
    case 'formalizations':
      return '/formalizations';
    case 'submit': {
      const params = new URLSearchParams();
      if (route.intake) params.set('type', route.intake);
      if (route.record && route.claim) {
        params.set('record', route.record);
        params.set('claim', route.claim);
      }
      return `/submit${params.size ? `?${params}` : ''}`;
    }
    case 'submissions':
      return '/submissions';
    case 'queue':
      return '/queue';
    case 'submission':
      return `/submissions/${seg(route.id)}`;
    case 'staff_submission':
      return `/staff/submissions/${seg(route.id)}`;
    case 'people':
      return '/people';
    case 'authors':
      return '/authors';
    case 'author':
      return `/authors/${seg(route.name)}`;
    case 'contribute':
      return '/contribute';
    case 'tasks': {
      const params = new URLSearchParams();
      if (route.submission) params.set('submission', route.submission);
      if (route.taskKind) params.set('kind', route.taskKind);
      const q = params.toString();
      return q === '' ? '/tasks' : `/tasks?${q}`;
    }
    case 'task':
      return `/tasks/${seg(route.id)}`;
    case 'contributors':
      return '/contributors';
    case 'not_found':
      return route.path;
  }
}
