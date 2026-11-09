import { expect, it, vi } from 'vitest';
import { fakeApi } from '../test/render';
import { dependencyPaper, dependencyKnowledge } from '../test/dependencyFixtures';
import type { StatementDetail } from '../api/types';
import { loadCollectedResult } from './publicResult';

const current: StatementDetail = {
  record: 'gap-note',
  title: dependencyPaper.title,
  publication: { version: 3, claims_revision: 1 },
  claim: {
    id: 's1',
    label: 'New lemma',
    kind: 'lemma',
    role: 'main',
    statement: 'A different result.',
    depends_on: [],
  },
  target: {
    claim: 's1',
    digest: 'new-target',
    toolchain: 'new-toolchain',
    lean: 'new Lean target',
  },
  macros: {},
  verified_attempts: [],
};
const signal = () => new AbortController().signal;
const apiForVersion = () =>
  fakeApi({
    getCollectedPaper: vi.fn(async () => dependencyPaper),
    collectedKnowledge: vi.fn(async () => dependencyKnowledge),
    getStatement: vi.fn(async () => current),
  });

it('retains the exact old statement without attaching the new target or current checked uses', async () => {
  const api = apiForVersion();
  const value = await loadCollectedResult(api, 'gap-note', 's1', '2', signal());
  expect(value.current).toBe(false);
  expect(value.result.claim.statement).toBe(dependencyKnowledge.statements[0]!.statement);
  expect(value.result.target).toBeNull();
  expect(value.result.verified_attempts).toEqual([]);
  expect(value.paper.revision).toBe('2');
  expect(api.collectedKnowledge).toHaveBeenCalledWith('gap-note', '2', expect.anything());
});
it('uses a current target only when the version and statement both match', async () => {
  const api = apiForVersion();
  api.getStatement = vi.fn(async () => ({
    ...current,
    publication: { version: 2, claims_revision: 1 },
    claim: { ...current.claim, statement: dependencyKnowledge.statements[0]!.statement },
  }));
  expect((await loadCollectedResult(api, 'gap-note', 's1', '2', signal())).result.target).toEqual(
    current.target,
  );
  api.getStatement = vi.fn(async () => ({
    ...current,
    publication: { version: 2, claims_revision: 1 },
  }));
  expect(
    (await loadCollectedResult(api, 'gap-note', 's1', '2', signal())).result.target,
  ).toBeNull();
});
it('does not substitute another public version when the requested one is unavailable', async () => {
  const api = apiForVersion();
  api.getCollectedPaper = vi.fn(async () => ({ ...dependencyPaper, revision: '3' }));
  await expect(loadCollectedResult(api, 'gap-note', 's1', '2', signal())).rejects.toThrow(
    /requested paper version/,
  );
  expect(api.getStatement).not.toHaveBeenCalled();
});
it('keeps a public historical statement readable if it has been removed from the current paper', async () => {
  const api = apiForVersion();
  api.getStatement = vi.fn(async () => {
    throw new Error('not found');
  });
  const value = await loadCollectedResult(api, 'gap-note', 's1', '2', signal());
  expect(value.result.claim.statement).toBe(dependencyKnowledge.statements[0]!.statement);
  expect(value.current).toBe(false);
});

it('uses the published section when an extracted result has no descriptive title', async () => {
  const api = apiForVersion();
  const statement = {
    ...dependencyKnowledge.statements[0]!,
    title: null,
    section: 'The conjecture and its counterexample',
  };
  api.collectedKnowledge = vi.fn(async () => ({ ...dependencyKnowledge, statements: [statement] }));
  api.getStatement = vi.fn(async () => ({
    ...current,
    publication: { version: 2, claims_revision: 1 },
    claim: { ...current.claim, statement: statement.statement },
  }));
  const result = await loadCollectedResult(api, 'gap-note', 's1', '2', signal());
  expect(result.result.claim.label).toBe('Lemma: The conjecture and its counterexample');
});
