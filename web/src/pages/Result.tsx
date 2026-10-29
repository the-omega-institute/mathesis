import { useCallback } from 'react';
import { useApi } from '../api/context';
import { useAsync } from '../api/useAsync';
import type { PublicPaper } from '../api/types';
import { AuthorLine } from '../components/AuthorLine';
import { Citation } from '../components/Citation';
import { ErrorReports, ReportError } from '../components/ErrorReports';
import { InlineLatex, LatexText, MacrosProvider } from '../components/Markdown';
import { ProofAttribution } from '../components/ProofAttribution';
import { Async } from '../components/ui';
import { isOpenKind } from '../lib/labels';
import { resultLabel } from '../lib/statements';
import { Link } from '../routing/router';
import { EntrantName } from './Leaderboard';
import { pdfHref } from '../api/client';

export function ResultPage({ record, claim }: { record: string; claim: string }) {
  const api = useApi();
  const load = useCallback(
    (signal: AbortSignal) => api.getPaper(record, { signal }),
    [api, record],
  );
  const paper = useAsync(load);
  return (
    <div className="page">
      <Async state={paper.state} onRetry={paper.reload}>
        {(value) => <ResultView paper={value} claimId={claim} />}
      </Async>
    </div>
  );
}

/** Read the immutable accepted paper; no Lean target is required to cite its mathematics. */
export function ResultView({ paper, claimId }: { paper: PublicPaper; claimId: string }) {
  const claim = paper.claims?.find((item) => item.id === claimId);
  if (!claim) return <p>This result is not available publicly.</p>;
  const record = paper.summary.record;
  const dependencies = paper.claims!.filter((item) => claim.depends_on.includes(item.id));
  const uses = dependencies.filter((item) => !isOpenKind(item.kind));
  const assumptions = dependencies.filter((item) => isOpenKind(item.kind));
  const usedBy = paper.claims!.filter((item) => item.depends_on.includes(claim.id));
  const version = paper.publication?.version ?? paper.versions?.at(-1)?.number;
  const witness = paper.new_content?.find((item) => item.claim === claim.id);
  const target = paper.lean_statements?.find((item) => item.claim === claim.id);
  const relations = (items: typeof uses) => (
    <ul>
      {items.map((item) => (
        <li key={item.id}>
          <Link to={{ kind: 'result', record, claim: item.id }}>
            <InlineLatex source={resultLabel(item)} />
          </Link>
        </li>
      ))}
    </ul>
  );
  return (
    <MacrosProvider macros={paper.macros ?? {}}>
      <article className="result-page">
        <p className="entry-meta">
          From{' '}
          <Link to={{ kind: 'paper', record }}>
            <InlineLatex source={paper.summary.title} />
          </Link>
          {version ? ` · version ${version}` : ''}
        </p>
        <h1>
          <InlineLatex source={resultLabel(claim)} />
        </h1>
        <AuthorLine authors={paper.summary.authors} />
        <LatexText source={claim.statement} className="result-statement" />
        {paper.summary.submission &&
        paper.versions?.find((item) => item.number === version)?.has_pdf ? (
          <p>
            <a href={pdfHref(paper.summary.submission, version)}>
              Read the proof in the paper (PDF)
            </a>
            {claim.section ? ` · ${claim.section}` : ''}
          </p>
        ) : null}
        <Citation paper={paper.summary} locator={{ claim, version }} />
        {witness?.lemmas.length ? (
          <section>
            <h2>Contribution</h2>
            {witness.lemmas.map((text, index) => (
              <LatexText key={index} source={text} />
            ))}
          </section>
        ) : null}
        {dependencies.length || usedBy.length ? (
          <section className="result-relations">
            <h2>Connections within this paper</h2>
            <p className="small muted">Dependencies stated in the manuscript.</p>
            {assumptions.length ? (
              <>
                <h3>Conditional on these assumptions</h3>
                <p>The argument assumes these open statements.</p>
                {relations(assumptions)}
              </>
            ) : null}
            {uses.length ? (
              <>
                <h3>Uses these results</h3>
                {relations(uses)}
              </>
            ) : null}
            {usedBy.length ? (
              <>
                <h3>
                  {isOpenKind(claim.kind) ? 'Assumed in these statements' : 'Used in these proofs'}
                </h3>
                {relations(usedBy)}
              </>
            ) : null}
          </section>
        ) : null}
        <section>
          <h2>Formalization</h2>
          {claim.verified ? (
            <>
              <p>
                <EntrantName entrant={claim.verified.entrant} /> proved the author-confirmed
                statement in Lean.
              </p>
              <ProofAttribution attribution={claim.verified.attribution} />
            </>
          ) : (
            <p>
              {claim.lean
                ? 'A verified formal proof is available.'
                : target
                  ? 'An author-confirmed statement is available for a community formal proof.'
                  : 'No public formal proof is available for this result.'}
            </p>
          )}
          {claim.verified || claim.lean || target ? (
            <Link to={{ kind: 'statement', record, claim: claim.id }}>
              View formalization and contributors
            </Link>
          ) : null}
        </section>
        <details className="history">
          <summary>Corrections</summary>
          <ReportError record={record} subject={{ kind: 'statement', claim: claim.id }} />
          <ErrorReports record={record} />
        </details>
      </article>
    </MacrosProvider>
  );
}
