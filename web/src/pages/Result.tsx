import { t } from '../i18n/locale';
import { ProofWork } from '../components/ProofWork';
import { useCallback } from 'react';
import { useApi } from '../api/context';
import { useAsync } from '../api/useAsync';
import type { PublicPaper } from '../api/types';
import { AuthorLine } from '../components/AuthorLine';
import { Citation } from '../components/Citation';
import { ErrorReports, ReportError } from '../components/ErrorReports';
import { InlineLatex, LatexText, MacrosProvider } from '../components/Markdown';
import { ProofAttribution } from '../components/ProofAttribution';
import { ProofBasisNote } from '../components/ProofBasis';
import { Async } from '../components/ui';
import { ResultConnections, CheckedResultUses } from '../components/ResultConnections';
import { ResultProvenance, matchesResult } from '../components/ResultProvenance';
import { safeHttpUrl } from '../lib/links';
import { resultLabel } from '../lib/statements';
import { Link } from '../routing/router';
import { EntrantName } from './Leaderboard';
import { pdfHref } from '../api/client';
import { loadCollectedResult } from '../lib/publicResult';
import { publishedLeanSource } from '../lib/publishedLean';

export function ResultPage({
  record,
  claim,
  version,
}: {
  record: string;
  claim: string;
  version?: string;
}) {
  if (!record.startsWith('WP-'))
    return <CollectedResultPage record={record} claim={claim} version={version} />;
  return <ReviewedResultPage record={record} claim={claim} />;
}
function ReviewedResultPage({ record, claim }: { record: string; claim: string }) {
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
  if (!claim) return <p>{t('This result is not available publicly.')}</p>;
  const record = paper.summary.record;
  const version = paper.publication?.version ?? paper.versions?.at(-1)?.number;
  const witness = paper.new_content?.find((item) => item.claim === claim.id);
  const target = paper.lean_statements?.find((item) => item.claim === claim.id);
  const sourceProof =
    paper.origin?.evidence.find(
      (item) => item.kind === 'lean_source' && matchesResult(claim, item.claim_label),
    )?.url ??
    paper.origin?.contributions?.find(
      (item) => item.role === 'formalization' && matchesResult(claim, item.claim_label),
    )?.evidence_url;
  const sourceProofHref = sourceProof ? safeHttpUrl(sourceProof) : null;
  return (
    <MacrosProvider macros={paper.macros ?? {}}>
      <article className="result-page">
        <p className="entry-meta">
          {t('From')}{' '}
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
            <a href={pdfHref(paper.summary.submission, version)}>{t('Read in the paper (PDF)')}</a>
            {claim.section ? ` · ${claim.section}` : ''}
          </p>
        ) : null}
        <Citation paper={paper.summary} locator={{ claim, version }} />
        {witness?.lemmas.length ? (
          <section>
            <h2>{t('Contribution')}</h2>
            {witness.lemmas.map((text, index) => (
              <LatexText key={index} source={text} />
            ))}
          </section>
        ) : null}
        <p>
          <Link to={{ kind: 'research_graph', focus: `paper:${record}` }}>
            {t('Explore the paper’s references and connections')}
          </Link>{' '}
          {t(
            'in the shared research graph. These citations describe the manuscript; they are not checked dependencies of this result.',
          )}
        </p>
        <ResultConnections paper={paper} claim={claim} />
        <CheckedResultUses record={record} claim={claim.id} />
        <ResultProvenance paper={paper} claim={claim} />
        <section>
          <h2>{t('Formalization')}</h2>
          {claim.verified ? (
            <>
              <p>
                <EntrantName entrant={claim.verified.entrant} />{' '}
                {t(' proved the author-confirmed statement in Lean.')}
              </p>
              <ProofAttribution attribution={claim.verified.attribution} />
              <ProofBasisNote basis={claim.verified.proof_basis} />
            </>
          ) : sourceProofHref && !claim.lean ? (
            <p>
              {t('Formalization is available from the original source.')}{' '}
              <a href={sourceProofHref} target="_blank" rel="noreferrer">
                {t('Read the Lean proof')}
              </a>
            </p>
          ) : (
            <p>
              {claim.lean
                ? t('A verified formal proof is available.')
                : target
                  ? t('An author-confirmed statement is available for a community formal proof.')
                  : t('No public formal proof is available for this result.')}
            </p>
          )}
          {claim.verified || claim.lean || target ? (
            <Link to={{ kind: 'statement', record, claim: claim.id }}>
              {t('View formalization and contributors')}
            </Link>
          ) : null}
        </section>
        {target ? <ProofWork record={record} claim={claim.id} /> : null}
        <details className="history">
          <summary>{t('Corrections')}</summary>
          <ReportError record={record} subject={{ kind: 'statement', claim: claim.id }} />
          <ErrorReports record={record} />
        </details>
      </article>
    </MacrosProvider>
  );
}

function CollectedResultPage({
  record,
  claim,
  version,
}: {
  record: string;
  claim: string;
  version?: string;
}) {
  const api = useApi();
  const load = useCallback(
    (signal: AbortSignal) => loadCollectedResult(api, record, claim, version, signal),
    [api, record, claim, version],
  );
  const data = useAsync(load);
  return (
    <div className="page">
      <Async state={data.state} onRetry={data.reload}>
        {({ paper, result, current }) => (
          <MacrosProvider macros={result.macros}>
            <article className="result-page">
              <p>
                <Link to={{ kind: 'paper', record, version: paper.revision }}>
                  <InlineLatex source={paper.title} />
                </Link>
              </p>
              <h1>
                <InlineLatex source={resultLabel(result.claim)} />
              </h1>
              <AuthorLine authors={paper.authors} />
              <LatexText source={result.claim.statement} className="result-statement" />
              <p>
                <a href={paper.pdf.url}>{t('Read in the paper (PDF)')}</a>
              </p>
              <section>
                <h2>{t('Formalization')}</h2>
                {result.verified_attempts.length ? (
                  result.verified_attempts.map((proof) => (
                    <div key={proof.id}>
                      <p>
                        {t(
                          proof.receipt.verdict === 'proved'
                            ? 'A verified formal proof is available.'
                            : 'A checked counterexample is available for this Lean statement.',
                        )}
                      </p>
                      {proof.attribution ? (
                        <ProofAttribution attribution={proof.attribution} />
                      ) : (
                        <p>
                          {t('Proof contribution by ')}
                          <EntrantName entrant={proof.entrant} />
                        </p>
                      )}
                      <ProofBasisNote
                        basis={proof.proof_basis}
                        verdict={proof.receipt.verdict === 'disproved' ? 'disproved' : 'proved'}
                      />
                      <a
                        href={`/api/v1/papers/${encodeURIComponent(record)}/statements/${encodeURIComponent(claim)}/attempts/${encodeURIComponent(proof.id)}/bundle`}
                      >
                        {t('Download proof and verification details')}
                      </a>
                      <details>
                        <summary>{t('View Lean proof')}</summary>
                        <pre>
                          <code>{proof.solution}</code>
                        </pre>
                      </details>
                    </div>
                  ))
                ) : publishedLeanSource(paper) ? (
                  <p>
                    {t(
                      'This paper includes published Lean formalization. Read its scope before proposing additional proof work; this manuscript statement has not yet been linked to a specific declaration.',
                    )}{' '}
                    <a href={publishedLeanSource(paper)}>{t('Read the Lean source')}</a>
                  </p>
                ) : (
                  <p>
                    {result.target
                      ? t('Contribute a proof of this exact statement.')
                      : t('No public formal proof is available for this result.')}
                  </p>
                )}
                {result.target ? (
                  <>
                    <details>
                      <summary>{t('View Lean statement')}</summary>
                      <pre>
                        <code>{result.target.lean}</code>
                      </pre>
                    </details>
                    <Link to={{ kind: 'submit', intake: 'lean', record, claim }}>
                      {t('Submit attributed Lean work')}
                    </Link>
                    {' · '}
                    <Link to={{ kind: 'statement', record, claim }}>
                      {t('View formalization and contributors')}
                    </Link>
                  </>
                ) : current ? (
                  <Link
                    to={{ kind: 'submit', intake: 'lean', record, claim, version: paper.revision }}
                  >
                    {t('Contribute Lean work')}
                  </Link>
                ) : null}
              </section>
              {current ? <CheckedResultUses record={record} claim={claim} /> : null}
              <Link
                to={{
                  kind: 'research_graph',
                  focus: `statement:${record}:${paper.revision}:${claim}`,
                }}
              >
                {t('Explore mathematical results')}
              </Link>
            </article>
          </MacrosProvider>
        )}
      </Async>
    </div>
  );
}
