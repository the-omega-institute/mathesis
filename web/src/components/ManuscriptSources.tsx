import type { ManuscriptOrigin } from '../api/types';
import { LatexText } from './Markdown';
import { ExternalLink } from './ui';
import { safeHttpUrl } from '../lib/links';

const labels = {
  written_proof: 'Written proof',
  finite_computation: 'Computational material',
  lean_source: 'Lean source',
};

export function ManuscriptSources({ origin }: { origin: ManuscriptOrigin }) {
  const source = safeHttpUrl(origin.source_url);
  return (
    <section aria-labelledby="manuscript-sources-h">
      <h2 id="manuscript-sources-h">Source and proof material</h2>
      <p>
        {source ? <ExternalLink href={source}>Original manuscript and sources</ExternalLink> : null}
        {' · '}
        {origin.license}
      </p>
      <p className="small muted">Related problems: {origin.problem_ids.join(', ')}</p>
      {origin.evidence.length ? (
        <ul className="entry-list">
          {origin.evidence.map((e, i) => {
            const url = safeHttpUrl(e.url);
            return (
              <li key={`${e.claim_label}-${i}`}>
                <strong>{e.claim_label}</strong>
                {' · '}
                {url ? <ExternalLink href={url}>{labels[e.kind]}</ExternalLink> : labels[e.kind]}
                <LatexText source={e.scope} />
              </li>
            );
          })}
        </ul>
      ) : null}
    </section>
  );
}
