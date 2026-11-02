import { useCallback, useId, useState } from 'react';
import { useApi } from '../api/context';
import { useAsync } from '../api/useAsync';
import type {
  CheckedContribution,
  FormalizationOpportunity,
  ResultLink,
  ResultUse,
} from '../api/types';
import { InlineLatex, LatexText } from './Markdown';
import { Async } from './ui';
import { Link } from '../routing/router';
import { EntrantName } from '../pages/Leaderboard';
import { ProofAttribution } from './ProofAttribution';

export function LeanKnowledgeGraph() {
  const api = useApi();
  const [before, setBefore] = useState<string | undefined>();
  const load = useCallback(
    async (signal: AbortSignal) => {
      const page = await api.formalizationOpportunities({ limit: 8, before }, { signal });
      // The existing public services enforce source visibility, target validity and proof receipts.
      const neighborhoods = await Promise.all(
        page.items.map(async (item) => {
          const [uses, checked] = await Promise.all([
            api.resultUses(item.record, item.claim, { signal }),
            api.checkedContributions(item.record, item.claim, { signal }),
          ]);
          return { uses, checked };
        }),
      );
      return {
        page,
        uses: neighborhoods.flatMap((n) => n.uses),
        checked: neighborhoods.flatMap((n) => n.checked),
      };
    },
    [api, before],
  );
  const graph = useAsync(load);
  return (
    <Async state={graph.state} onRetry={graph.reload}>
      {(data) => (
        <>
          <LeanGraphView
            key={before ?? 'first'}
            items={data.page.items}
            uses={data.uses}
            checked={data.checked}
          />
          {data.page.next_before ? (
            <p>
              <button
                type="button"
                className="text-button"
                onClick={() => setBefore(data.page.next_before!)}
              >
                More formal results
              </button>
            </p>
          ) : null}
          {before ? (
            <p>
              <button type="button" className="text-button" onClick={() => setBefore(undefined)}>
                Back to the first results
              </button>
            </p>
          ) : null}
        </>
      )}
    </Async>
  );
}

type Node = {
  id: string;
  title: string;
  context: ResultLink | FormalizationOpportunity;
  checked?: CheckedContribution;
  use?: ResultUse;
  item?: FormalizationOpportunity;
};
const resultKey = (r: { record: string; claim: string }) => JSON.stringify([r.record, r.claim]);
const propositionKey = (attempt: string, name: string) => JSON.stringify([attempt, name]);

/** Every arrow denotes actual use of an exact contributed declaration in a checked proof term. */
export function LeanGraphView({
  items,
  uses,
  checked,
}: {
  items: FormalizationOpportunity[];
  uses: ResultUse[];
  checked: CheckedContribution[];
}) {
  const arrowId = useId();
  const panelId = useId();
  const results = new Map<string, Node>();
  const propositions = new Map<string, Node>();
  for (const item of items)
    results.set(resultKey(item), {
      id: resultKey(item),
      title: `${item.label.replace(/\s*\[[^\]]*\]\s*$/, '')} — ${item.title}`,
      context: item,
      item,
    });
  for (const contribution of checked) {
    const context = results.get(resultKey(contribution.origin))?.context;
    if (!context) continue;
    const id = propositionKey(contribution.origin.attempt, contribution.intermediate.name);
    propositions.set(id, {
      id,
      title: contribution.intermediate.title,
      context,
      checked: contribution,
    });
  }
  const edges = new Map<string, { from: string; to: string; use: ResultUse }>();
  for (const use of uses) {
    const from = propositionKey(use.source_attempt, use.contribution.name);
    const to = resultKey(use.result);
    if (!propositions.has(from))
      propositions.set(from, {
        id: from,
        title: use.contribution.title,
        context: use.used_result,
        use,
      });
    if (!results.has(to))
      results.set(to, {
        id: to,
        title: `${use.result.label.replace(/\s*\[[^\]]*\]\s*$/, '')} — ${use.result.title}`,
        context: use.result,
        use,
      });
    edges.set(JSON.stringify([from, to, use.attempt]), { from, to, use });
  }
  const nodes = [...propositions.values(), ...results.values()];
  const [selection, setSelection] = useState<string | undefined>();
  const selected = nodes.find((n) => n.id === selection) ?? nodes[0];
  const height = Math.max(320, 64 + 128 * Math.max(propositions.size, results.size));
  const positions = new Map<string, { x: number; y: number }>();
  for (const [column, group] of [[...propositions.values()], [...results.values()]].entries()) {
    group.forEach((node, index) =>
      positions.set(node.id, { x: column ? 460 : 16, y: 48 + 128 * index }),
    );
  }
  if (!selected)
    return (
      <div className="research-map-empty">
        <h2>No public Lean results yet</h2>
        <p>
          Author-confirmed statements, checked propositions and their proof connections will appear
          here as research becomes public.
        </p>
        <Link to={{ kind: 'submit', intake: 'lean' }}>Contribute Lean work</Link>
      </div>
    );
  const neighbors = [...edges.values()].filter(
    (e) => e.from === selected.id || e.to === selected.id,
  );
  const contribution = selected.checked;
  const use = selected.use ?? neighbors[0]?.use;
  return (
    <div className="research-map-layout">
      <figure className="research-map" aria-label="Lean knowledge connections">
        <div
          className="research-map-scroll"
          role="region"
          tabIndex={0}
          aria-label="Scrollable Lean knowledge graph"
        >
          <div className="research-map-canvas" style={{ height }}>
            <span className="research-map-column" style={{ left: '2.2%' }}>
              Checked propositions
            </span>
            <span className="research-map-column" style={{ left: '63%' }}>
              Results & formalization targets
            </span>
            <svg viewBox={`0 0 730 ${height}`} preserveAspectRatio="none" aria-hidden="true">
              <defs>
                <marker
                  id={arrowId}
                  viewBox="0 0 10 10"
                  refX="9"
                  refY="5"
                  markerWidth="6"
                  markerHeight="6"
                  orient="auto"
                >
                  <path d="M 0 0 L 10 5 L 0 10 z" fill="context-stroke" />
                </marker>
              </defs>
              {[...edges.values()].map((edge) => {
                const a = positions.get(edge.from)!;
                const b = positions.get(edge.to)!;
                return (
                  <path
                    key={JSON.stringify([edge.from, edge.to, edge.use.attempt])}
                    className={`research-map-edge${edge.from === selected.id || edge.to === selected.id ? ' is-active' : ''}`}
                    d={`M 274 ${a.y + 48} C 354 ${a.y + 48}, 370 ${b.y + 48}, 450 ${b.y + 48}`}
                    markerEnd={`url(#${arrowId})`}
                  />
                );
              })}
            </svg>
            {nodes.map((node) => {
              const position = positions.get(node.id)!;
              return (
                <button
                  key={node.id}
                  type="button"
                  className={`research-map-node${selected.id === node.id ? ' is-connected' : ''}`}
                  style={{ left: `${(position.x / 730) * 100}%`, top: position.y }}
                  aria-pressed={selected.id === node.id}
                  aria-controls={panelId}
                  onClick={() => setSelection(node.id)}
                >
                  <span className="research-map-node-title">
                    <InlineLatex source={node.title} />
                  </span>
                  <span className="research-map-node-authors">
                    {node.checked || propositions.has(node.id)
                      ? 'Lean checked'
                      : node.item?.status === 'open'
                        ? 'Awaiting formal proof'
                        : node.item?.status === 'disproved'
                          ? 'Lean disproof'
                          : 'Lean checked'}
                  </span>
                </button>
              );
            })}
          </div>
        </div>
        <figcaption>
          Arrows show which contributed propositions a checked proof actually uses. Select a result
          to see its sources and contributors.
        </figcaption>
      </figure>
      <section
        id={panelId}
        className="research-map-detail"
        aria-label="Selected Lean result"
        role="region"
      >
        <h2>
          <InlineLatex source={selected.title} />
        </h2>
        <p className="research-map-credit">
          {selected.context.authors.map((a) => a.name).join('; ')}
        </p>
        <p>
          <Link
            to={{ kind: 'result', record: selected.context.record, claim: selected.context.claim }}
          >
            Read the mathematics and contribution history
          </Link>
        </p>
        {contribution ? (
          <>
            <LatexText source={contribution.intermediate.proposition} />
            <p>
              Formalization contributed by <EntrantName entrant={contribution.contributor} />.
            </p>
            <ProofAttribution attribution={contribution.attribution ?? undefined} />
          </>
        ) : selected.item ? (
          <>
            <LatexText source={selected.item.statement} />
            {selected.item.contributor ? (
              <p>
                Verified contribution by <EntrantName entrant={selected.item.contributor} />.
              </p>
            ) : (
              <p>This confirmed Lean target is open for contributions.</p>
            )}
            <ProofAttribution attribution={selected.item.attribution ?? undefined} />
          </>
        ) : null}
        {!contribution && propositions.has(selected.id) && use ? (
          <>
            <LatexText source={use.contribution.proposition} />
            <p>
              Formalization contributed by <EntrantName entrant={use.contributor} />.
            </p>
            <ProofAttribution attribution={use.attribution ?? undefined} />
          </>
        ) : null}
        {neighbors.length ? (
          <>
            <h3>{propositions.has(selected.id) ? 'A stepping stone for' : 'Builds on'} </h3>
            <ul>
              {neighbors.map((edge) => {
                const other = nodes.find(
                  (n) => n.id === (edge.from === selected.id ? edge.to : edge.from),
                )!;
                return (
                  <li key={JSON.stringify([edge.from, edge.to, edge.use.attempt])}>
                    <button
                      type="button"
                      className="text-button"
                      onClick={() => setSelection(other.id)}
                    >
                      <InlineLatex source={other.title} />
                    </button>
                  </li>
                );
              })}
            </ul>
          </>
        ) : null}
        {use ? (
          <p>
            <a
              href={`/api/v1/papers/${encodeURIComponent(use.result.record)}/statements/${encodeURIComponent(use.result.claim)}/attempts/${encodeURIComponent(use.attempt)}/bundle`}
            >
              Read the checked proof and its sources
            </a>
          </p>
        ) : null}
        {contribution ? (
          <p>
            <a
              href={`/api/v1/papers/${encodeURIComponent(contribution.origin.record)}/statements/${encodeURIComponent(contribution.origin.claim)}/attempts/${encodeURIComponent(contribution.origin.attempt)}/bundle`}
            >
              Read this proposition’s checked source
            </a>
          </p>
        ) : null}
        <p className="small muted">
          This view follows checked declarations. A useful intermediate proposition can be verified
          while its source problem remains open.
        </p>
      </section>
    </div>
  );
}
