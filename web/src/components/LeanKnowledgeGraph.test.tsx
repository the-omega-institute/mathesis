import { screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, it, vi } from 'vitest';
import type { CheckedContribution, FormalizationOpportunity, ResultUse } from '../api/types';
import { fakeApi, renderWithApp } from '../test/render';
import { LeanKnowledgeGraph, LeanGraphView } from './LeanKnowledgeGraph';
const receipt = {
  verdict: 'proved' as const,
  reason: 'Independent verifier checked the proposition',
  target_digest: 'target',
  solution_digest: 'source',
  toolchain: 'pinned-toolchain',
  axioms: [],
  checked_at: '2026-11-02T00:00:00Z',
  duration: 1,
};

const author = { name: 'Earlier mathematician' };
const contributor = {
  id: 'formalizer',
  name: 'Formalizer',
  kind: 'person' as const,
  retired: false,
  revision: 0,
};
const item: FormalizationOpportunity = {
  record: 'WP-2026-0001',
  claim: 'C1',
  title: 'An open problem',
  label: 'Conjecture 1',
  statement: 'A precise statement',
  authors: [author],
  kind: 'conjecture',
  status: 'open',
  target_digest: 'target',
  toolchain: 'pinned-toolchain',
  verified_count: 0,
  contributor: null,
  attribution: null,
  receipt: null,
};
const helper: CheckedContribution = {
  origin: {
    record: item.record,
    claim: item.claim,
    version: 1,
    claims_revision: 1,
    attempt: 'helper-proof',
    target_digest: 'target',
    solution_digest: 'source',
    declaration: 'Research.bound',
  },
  intermediate: {
    name: 'Research.bound',
    title: 'A useful bound',
    proposition: 'True',
    license: 'CC-BY-4.0',
  },
  authors: [author],
  contributor,
  attribution: null,
  source_sha256: 'source',
  receipt,
  verification_request: {
    target: 'target',
    target_digest: 'target',
    toolchain: 'pin',
    solution: 'proof',
    modules: {},
  },
};
const use: ResultUse = {
  result: {
    record: 'WP-2026-0002',
    claim: 'C2',
    title: 'A later theorem',
    label: 'Theorem 2',
    authors: [{ name: 'Later mathematician' }],
    version: 1,
    claims_revision: 1,
  },
  used_result: {
    record: item.record,
    claim: item.claim,
    title: item.title,
    label: item.label,
    authors: item.authors,
    version: 1,
    claims_revision: 1,
  },
  contribution: helper.intermediate,
  contributor,
  attribution: null,
  attempt: 'later-proof',
  source_attempt: helper.origin.attempt,
  basis: 'checked_proof_term',
  joint_context: [helper.origin],
};

it('shows a checked intermediate proposition without marking its open source conjecture proved', async () => {
  renderWithApp(<LeanGraphView items={[item]} checked={[helper]} uses={[use, use]} />, fakeApi());
  const map = screen.getByRole('figure', { name: 'Lean knowledge connections' });
  expect(map.querySelectorAll('.research-map-edge')).toHaveLength(1);
  expect(within(map).getByRole('button', { name: /Conjecture 1/ })).toHaveTextContent(
    'Awaiting formal proof',
  );
  expect(within(map).getByRole('button', { name: /A useful bound/ })).toHaveTextContent(
    'Lean checked',
  );
  const details = screen.getByRole('region', { name: 'Selected Lean result' });
  expect(details).toHaveTextContent('Earlier mathematician');
  expect(details).toHaveTextContent('Formalizer');
  expect(
    within(details).getByRole('link', { name: 'Read this proposition’s checked source' }),
  ).toHaveAttribute(
    'href',
    '/api/v1/papers/WP-2026-0001/statements/C1/attempts/helper-proof/bundle',
  );
  await userEvent.click(
    within(details).getByRole('button', { name: 'Theorem 2 — A later theorem' }),
  );
  expect(screen.getByRole('region', { name: 'Selected Lean result' })).toHaveTextContent(
    'Later mathematician',
  );
});

it('includes an unused checked proposition without inventing a dependency arrow', () => {
  renderWithApp(<LeanGraphView items={[item]} checked={[helper]} uses={[]} />, fakeApi());
  const map = screen.getByRole('figure', { name: 'Lean knowledge connections' });
  expect(map.querySelectorAll('.research-map-edge')).toHaveLength(0);
  expect(within(map).getByRole('button', { name: /A useful bound/ })).toBeVisible();
});

it('loads a bounded page through public proof services and follows its cursor', async () => {
  const listing = vi.fn(async () => ({ items: [item], next_before: 'next-target' }));
  const api = fakeApi({
    formalizationOpportunities: listing,
    checkedContributions: vi.fn(async () => [helper]),
    resultUses: vi.fn(async () => [use]),
  });
  renderWithApp(<LeanKnowledgeGraph />, api);
  await screen.findByRole('figure', { name: 'Lean knowledge connections' });
  expect(listing).toHaveBeenCalledWith({ limit: 8, before: undefined }, expect.anything());
  expect(api.checkedContributions).toHaveBeenCalledWith(
    item.record,
    item.claim,
    expect.objectContaining({ signal: expect.any(AbortSignal) }),
  );
  await userEvent.click(screen.getByRole('button', { name: 'More formal results' }));
  await screen.findByRole('button', { name: 'Back to the first results' });
  expect(listing).toHaveBeenLastCalledWith({ limit: 8, before: 'next-target' }, expect.anything());
});

it('shows collected Lean sources with original authors and no invented checked-use edges', () => {
  renderWithApp(
    <LeanGraphView
      items={[]}
      uses={[]}
      checked={[]}
      sources={[
        {
          id: 'a-note',
          revision: '2',
          title: 'A collected theorem',
          abstract: 'A result',
          authors: [{ name: 'Original mathematician' }],
          license: 'CC-BY-4.0',
          msc: [],
          problems: [],
          published_on: '2026-11-05T00:00:00Z',
          lean: {
            url: `https://github.com/example/proofs/blob/${'a'.repeat(40)}/Proof.lean`,
            revision: 'a'.repeat(40),
          },
          pdf: { url: '/paper.pdf', bytes: 1, sha256: 'a'.repeat(64) },
          source: { url: '/source.zip', bytes: 1, sha256: 'b'.repeat(64) },
        },
      ]}
    />,
    fakeApi(),
  );
  expect(screen.getByText('Published Lean source')).toBeVisible();
  const panel = screen.getByRole('region', { name: 'Selected Lean result' });
  expect(within(panel).getByText('Original mathematician')).toBeVisible();
  expect(
    within(panel).getByRole('link', { name: 'Read the mathematics and contribution history' }),
  ).toHaveAttribute('href', '/papers/a-note?version=2');
  expect(within(panel).getByRole('link', { name: 'Read the Lean source' })).toHaveAttribute(
    'href',
    expect.stringContaining('/Proof.lean'),
  );
  expect(document.querySelectorAll('.research-map-edge')).toHaveLength(0);
  expect(screen.queryByText('Lean checked')).not.toBeInTheDocument();
  expect(
    screen.getByText('Confirmed targets will appear here for community contributions.'),
  ).toBeVisible();
});
