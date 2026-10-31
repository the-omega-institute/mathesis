import { useCallback, useState } from 'react';
import { useApi } from '../api/context';
import { useAsync } from '../api/useAsync';
import type { ResearchGraph, ResearchNode } from '../api/types';
import { Async } from '../components/ui';
import { safeHttpUrl } from '../lib/links';
import { Link, useRouter } from '../routing/router';

export function ResearchGraphPage({
  focus,
  q,
  before,
}: {
  focus?: string;
  q?: string;
  before?: string;
}) {
  const api = useApi();
  const { navigate } = useRouter();
  const [search, setSearch] = useState(q ?? '');
  const load = useCallback(
    (signal: AbortSignal) => api.researchGraph({ focus, q, before }, { signal }),
    [api, focus, q, before],
  );
  const graph = useAsync(load);
  return (
    <div className="page research-graph">
      <h1>Research graph</h1>
      <p>
        Explore published manuscripts and the earlier literature they cite. A shared reference can
        connect work across papers and sources.
      </p>
      <form
        className="archive-search"
        onSubmit={(e) => {
          e.preventDefault();
          navigate({ kind: 'research_graph', ...(search.trim() ? { q: search.trim() } : {}) });
        }}
      >
        <label htmlFor="graph-search">Find a work, identifier or cited author</label>
        <input
          id="graph-search"
          value={search}
          maxLength={300}
          onChange={(e) => setSearch(e.target.value)}
        />
        <button type="submit">Search graph</button>
      </form>
      {focus || q || before ? (
        <p>
          <Link to={{ kind: 'research_graph' }}>Browse from the beginning</Link>
        </p>
      ) : null}
      <Async state={graph.state} onRetry={graph.reload}>
        {(value) => <GraphView graph={value} focus={focus} q={q} />}
      </Async>
    </div>
  );
}
function Work({ node }: { node: ResearchNode }) {
  const href = node.url ? safeHttpUrl(node.url) : null;
  return (
    <>
      <Link to={{ kind: 'research_graph', focus: node.id }}>{node.title}</Link>
      {node.authors.length ? <p>{node.authors.join('; ')}</p> : null}
      <p className="entry-meta">
        {node.kind === 'literature'
          ? 'Earlier work described in a citation'
          : 'Published manuscript'}
        {node.record ? (
          <>
            {' '}
            · <Link to={{ kind: 'paper', record: node.record }}>Read paper and results</Link>
          </>
        ) : null}
        {href ? (
          <>
            {' '}
            ·{' '}
            <a href={href} target="_blank" rel="noreferrer">
              Original source
            </a>
          </>
        ) : null}
      </p>
    </>
  );
}
export function GraphView({
  graph,
  focus,
  q,
}: {
  graph: ResearchGraph;
  focus?: string;
  q?: string;
}) {
  const nodes = new Map(graph.nodes.map((node) => [node.id, node]));
  const focused = focus ? nodes.get(focus) : undefined;
  return (
    <>
      <p className="muted">Public manuscripts and the earlier works named in their citations.</p>
      {focused ? (
        <section className="graph-focus">
          <h2>{focused.kind === 'literature' ? 'Selected earlier work' : 'Selected manuscript'}</h2>
          <Work node={focused} />
        </section>
      ) : null}
      {graph.nodes.length && !focus && !graph.edges.length ? (
        <section>
          <h2>Works in this view</h2>
          <ul className="graph-work-list">
            {graph.nodes
              .filter((n) => n.id !== focus)
              .map((node) => (
                <li key={node.id}>
                  <Work node={node} />
                </li>
              ))}
          </ul>
        </section>
      ) : null}
      {!graph.nodes.length ? (
        <p>
          No matching public work was found in the indexed accepted sources. Reference coverage may
          be incomplete.
        </p>
      ) : null}
      {graph.edges.length ? (
        <section>
          <h2>
            {focused?.kind === 'literature'
              ? 'Cited by'
              : focused?.kind === 'manuscript'
                ? 'References in this manuscript'
                : 'Relationships in this view'}
          </h2>
          <p>
            Each relationship comes from the citing manuscript. Names and descriptions stay as
            supplied; matching names do not identify a person. A citation does not establish a
            checked proof dependency or transfer proof credit.
          </p>
          <ol className="graph-citations">
            {graph.edges.map((edge) => (
              <li key={`${edge.record}:${edge.version}:${edge.reference_key}`}>
                <Link to={{ kind: 'paper', record: edge.record }}>
                  {nodes.get(edge.from)?.title ?? edge.record}
                </Link>
                {' cites '}
                <Link to={{ kind: 'research_graph', focus: edge.to }}>
                  {edge.title?.trim() ||
                    edge.display_text ||
                    nodes.get(edge.to)?.title ||
                    'Cited work'}
                </Link>
                {edge.authors.length ? (
                  <p>
                    {edge.authors.join('; ')}
                    {edge.year ? ` (${edge.year})` : ''}
                  </p>
                ) : null}
                <p className="entry-meta">Source-declared citation from this manuscript.</p>
                <details>
                  <summary>Source evidence</summary>
                  <p className="reference-evidence">{edge.raw}</p>
                  <p>
                    Reference key {edge.reference_key} · {edge.source_location} · version{' '}
                    {edge.version}
                    {edge.raw_truncated ? ' · bounded prefix shown' : ''}
                  </p>
                </details>
              </li>
            ))}
          </ol>
        </section>
      ) : null}
      {graph.next_before ? (
        <p>
          <Link to={{ kind: 'research_graph', focus, q, before: graph.next_before }}>
            Next page of matching works
          </Link>
        </p>
      ) : null}
      <details>
        <summary>About coverage</summary>
        <p>
          {graph.scope} Follow the next-page link when one is available. Only currently public
          accepted sources contribute descriptions.
        </p>
        <ul>
          {graph.coverage.map((c) => (
            <li key={c.record}>
              <Link to={{ kind: 'paper', record: c.record }}>{c.record}</Link> · version {c.version}
              {!c.method
                ? ' · not indexed'
                : c.method === 'pdf_unindexed_v1'
                  ? ' · PDF references not indexed'
                  : ' · literal source citations'}
              <ul>
                {c.limitations.map((s) => (
                  <li key={s}>{s}</li>
                ))}
              </ul>
            </li>
          ))}
        </ul>
      </details>
    </>
  );
}
