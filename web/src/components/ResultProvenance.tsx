import type { PublicClaim, PublicPaper, SourceContribution } from '../api/types';
import { safeHttpUrl } from '../lib/links';
import { ExternalLink } from './ui';
import { LatexText } from './Markdown';

export const contributionRoles: Record<SourceContribution['role'], string> = {
  problem: 'Original problem',
  mathematical_proof: 'Mathematical proof',
  exposition: 'Exposition',
  formalization: 'Formalization',
  correction: 'Correction',
};

export function matchesResult(claim: PublicClaim, label: string) {
  return label === claim.id || label === claim.label || label === claim.latex_label;
}

/** Source history is acyclic on admission. The fallback preserves legacy data
 * without claiming an order when a third-party response is malformed. */
function orderedContributions(items: SourceContribution[]) {
  const pending = [...items];
  const known = new Set(items.map((item) => item.id));
  const ordered: SourceContribution[] = [];
  while (pending.length) {
    const next = pending.findIndex((item) =>
      (item.depends_on ?? []).every((id) => !known.has(id) || ordered.some((i) => i.id === id)),
    );
    if (next < 0) return items;
    ordered.push(...pending.splice(next, 1));
  }
  return ordered;
}

function EvidenceLink({ url, children }: { url: string; children: React.ReactNode }) {
  const href = safeHttpUrl(url);
  return href ? <ExternalLink href={href}>{children}</ExternalLink> : <>{children}</>;
}

export function ResultProvenance({ paper, claim }: { paper: PublicPaper; claim: PublicClaim }) {
  const origin = paper.origin;
  if (!origin) return null;
  const contributions = orderedContributions(origin.contributions ?? []).filter((item) =>
    matchesResult(claim, item.claim_label),
  );
  const evidence = origin.evidence.filter((item) => matchesResult(claim, item.claim_label));
  return (
    <section className="result-provenance" aria-labelledby="result-provenance-heading">
      <h2 id="result-provenance-heading">Source and contributions</h2>
      <p>
        <EvidenceLink url={origin.source_url}>Original research</EvidenceLink>
        {' · '}
        {origin.license}
      </p>
      {contributions.length ? (
        <>
          <p className="small muted">Contributions attributed in the original source.</p>
          <ul className="contribution-history">
            {contributions.map((item) => (
              <li key={item.id}>
                <h3>{contributionRoles[item.role]}</h3>
                <div>
                  <p className="contribution-names">
                    {item.contributors.map((person, index) => (
                      <span key={index}>
                        {index ? ', ' : ''}
                        {person.profile_url ? (
                          <EvidenceLink url={person.profile_url}>{person.name}</EvidenceLink>
                        ) : (
                          person.name
                        )}
                      </span>
                    ))}
                  </p>
                  <LatexText source={item.scope} />
                  <p className="small">
                    <EvidenceLink url={item.evidence_url}>
                      {item.role === 'formalization' ? 'Lean source' : 'Read the evidence'}
                    </EvidenceLink>
                  </p>
                  {(item.depends_on ?? []).length ? (
                    <p className="small">
                      Builds on:{' '}
                      {(item.depends_on ?? []).map((id, index) => {
                        const parent = origin.contributions?.find((entry) => entry.id === id);
                        return parent ? (
                          <span key={id}>
                            {index ? '; ' : ''}
                            <EvidenceLink url={parent.evidence_url}>
                              {contributionRoles[parent.role]} —{' '}
                              {parent.contributors.map((p) => p.name).join(', ')}
                            </EvidenceLink>
                          </span>
                        ) : null;
                      })}
                    </p>
                  ) : null}
                </div>
              </li>
            ))}
          </ul>
        </>
      ) : null}
      {evidence.length ? (
        <details className="source-evidence">
          <summary>Supporting sources</summary>
          <ul>
            {evidence.map((item, index) => (
              <li key={index}>
                <EvidenceLink url={item.url}>
                  {
                    {
                      written_proof: 'Written proof',
                      finite_computation: 'Finite computation',
                      lean_source: 'Lean source',
                    }[item.kind]
                  }
                </EvidenceLink>
                <LatexText source={item.scope} />
              </li>
            ))}
          </ul>
        </details>
      ) : null}
    </section>
  );
}
