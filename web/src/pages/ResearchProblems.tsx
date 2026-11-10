import { t } from '../i18n/locale';
import { useCallback } from 'react';
import { useApi } from '../api/context';
import { useAsync } from '../api/useAsync';
import { usePaged } from '../api/usePaged';
import type {
  CollectedProblemNote,
  CollectedProof,
  CuratedProblem,
  ResearchProblemLink,
  ResearchProblemSummary,
  SourceContribution,
} from '../api/types';
import { AuthorLine } from '../components/AuthorLine';
import { InlineLatex, LatexText } from '../components/Markdown';
import { Pager } from '../components/Pager';
import { Async, DateText, ExternalLink, SourceLink } from '../components/ui';
import { formatDate } from '../lib/format';
import { safeHttpUrl, sourceHref, sourceLabel } from '../lib/links';
import { resultLabel } from '../lib/statements';
import { Link } from '../routing/router';

type CuratedSummaryStatus = 'curated_open' | 'curated_partially_resolved';
const statuses: Record<Exclude<ResearchProblemSummary['status'], CuratedSummaryStatus>, string> = {
  related_research: 'Reviewed research',
  settled: 'Settled',
  partial: 'Partial progress',
  conflicting: 'Conflicting conclusions',
};
/** A curator's reading is always shown with the day its evidence was last checked. */
const curatedReadings: Record<CuratedProblem['status'], string> = {
  open: 'Open as of {date}',
  partially_resolved: 'Partially resolved as of {date}',
};
function curatedReading(status: CuratedProblem['status'], asOf: string): string {
  return t(curatedReadings[status], { date: formatDate(asOf) });
}
function statusText(summary: ResearchProblemSummary): string {
  if (summary.status !== 'curated_open' && summary.status !== 'curated_partially_resolved')
    return t(statuses[summary.status]);
  const status = summary.status === 'curated_open' ? 'open' : 'partially_resolved';
  return `${t('Curated')} · ${curatedReading(status, summary.status_as_of ?? '')}`;
}
const roles: Record<SourceContribution['role'], string> = {
  problem: 'Original problem',
  mathematical_proof: 'Mathematical proof',
  exposition: 'Exposition',
  formalization: 'Formalization',
  correction: 'Correction',
};
const leanStates: [CollectedProof['state'], string][] = [
  ['verified', 'Verified in Lean'],
  ['disproved', 'Disproved in Lean'],
  ['open', 'Open for formalization'],
];

/** A collected work's published Lean source and its independently checked results. */
function CollectedLeanStatus({ note }: { note: CollectedProblemNote }) {
  const api = useApi();
  const version = String(note.version);
  const load = useCallback(
    (signal: AbortSignal) => api.collectedKnowledge(note.work, version, { signal }),
    [api, note.work, version],
  );
  const rows = useAsync(load).value?.formalizations ?? [];
  const source = note.lean ? safeHttpUrl(note.lean.url) : null;
  const checked = leanStates
    .map(([state, label]) => [label, rows.filter((row) => row.state === state).length] as const)
    .filter(([, count]) => count > 0);
  if (!source && !checked.length) return null;
  return (
    <p>
      {t('Lean status:')}{' '}
      {source ? <ExternalLink href={source}>{t('Lean source')}</ExternalLink> : null}
      {checked.map(([label, count], index) => (
        <span key={label}>
          {source || index ? ' · ' : ''}
          {t(label)}: {count}
        </span>
      ))}
    </p>
  );
}

function CollectedNote({ note }: { note: CollectedProblemNote }) {
  const paper = { kind: 'paper', record: note.work, version: String(note.version) } as const;
  return (
    <article className="paper-entry">
      <h3>
        <Link to={paper}>
          <InlineLatex source={note.title} />
        </Link>
      </h3>
      <AuthorLine authors={note.authors} />
      <p className="entry-meta">
        {t('Version ')}
        {note.version} · <DateText iso={note.published_on} /> · {note.license}
      </p>
      <p>
        <Link to={paper}>{t('Read full text and version history')}</Link>
      </p>
      <CollectedLeanStatus note={note} />
    </article>
  );
}

function Posers({ people }: { people: { name: string; profile_url?: string }[] }) {
  return (
    <p>
      {t('Proposed by')}{' '}
      {people.map((person, i) => {
        const url = person.profile_url && safeHttpUrl(person.profile_url);
        return (
          <span key={`${person.name}-${i}`}>
            {i ? '; ' : ''}
            {url ? <ExternalLink href={url}>{person.name}</ExternalLink> : person.name}
          </span>
        );
      })}
    </p>
  );
}

/** A directly collected problem: its statement, credit, and the curator's dated reading. */
function CuratedEntry({ id, problem: c }: { id: string; problem: CuratedProblem }) {
  return (
    <>
      <section aria-label={t('Problem statement')}>
        <h2>{t('Statement')}</h2>
        <LatexText source={c.statement} />
        {c.posers.length ? (
          <Posers people={c.posers} />
        ) : (
          <p>
            {t('Original poser not recorded:')} {c.posers_unknown}
          </p>
        )}
        <p>
          <SourceLink source={c.source} />
        </p>
        {c.msc.length || c.directions.length ? (
          <p className="entry-meta">
            {[...c.msc.map((code) => `MSC ${code}`), ...c.directions].join(' · ')}
          </p>
        ) : null}
      </section>
      {c.background ? (
        <section aria-labelledby="problem-background-h">
          <h2 id="problem-background-h">{t('Background')}</h2>
          <LatexText source={c.background} />
        </section>
      ) : null}
      <section aria-labelledby="problem-status-h">
        <h2 id="problem-status-h">{curatedReading(c.status, c.status_as_of)}</h2>
        <p>
          {t(
            'This status is the curator’s reading of the sources below on the dates shown. It is not a Mathesis review or audit result.',
          )}
        </p>
        <ul>
          {c.status_evidence.map((evidence, index) => {
            const url = safeHttpUrl(evidence.url);
            return (
              <li key={`${evidence.url}-${index}`}>
                {url ? <ExternalLink href={url}>{evidence.url}</ExternalLink> : evidence.url}
                {' · '}
                {t('Checked on')}{' '}
                <time dateTime={evidence.checked_on}>{formatDate(evidence.checked_on)}</time>
                {' · '}
                {evidence.observed}
              </li>
            );
          })}
        </ul>
      </section>
      <section aria-labelledby="problem-participate-h">
        <h2 id="problem-participate-h">{t('Work on this problem')}</h2>
        <p>
          {t(
            'A written solution goes through ordinary review. Lean work is saved with the original posers credited; no Lean statement has been prepared for this problem yet.',
          )}
        </p>
        <p>
          <Link to={{ kind: 'submit' }}>{t('Submit a written solution')}</Link>
          {' · '}
          <Link to={{ kind: 'submit', intake: 'lean', problem: id }}>
            {t('Contribute a Lean formalization')}
          </Link>
        </p>
      </section>
    </>
  );
}

export function ProblemLinks({ links }: { links: ResearchProblemLink[] }) {
  if (!links.length) return null;
  return (
    <section aria-label={t('Related problems')}>
      <h2>{t('Related problems')}</h2>
      <ul>
        {links.map((p) => (
          <li key={p.id}>
            <Link to={{ kind: 'research_problem', id: p.id }}>
              <InlineLatex source={p.title} />
            </Link>
          </li>
        ))}
      </ul>
    </section>
  );
}

export function ResearchProblems() {
  const api = useApi();
  const load = useCallback(
    (before: string | null, signal: AbortSignal) =>
      api.listResearchProblems({ before, limit: 25 }, { signal }),
    [api],
  );
  const paged = usePaged(load);
  return (
    <>
      <p>{t('Original problems and the papers and notes that build on them.')}</p>
      <Async state={paged.state} onRetry={paged.reload}>
        {(page) =>
          page.items.length ? (
            <ul className="entry-list">
              {page.items.map((p) => (
                <li key={p.id}>
                  <Link className="entry-title" to={{ kind: 'research_problem', id: p.id }}>
                    <InlineLatex source={p.title} />
                  </Link>
                  <p className="entry-meta">
                    {statusText(p)} · {p.notes}{' '}
                    {p.notes === 1 ? t('paper or note') : t('papers and notes')}
                  </p>
                </li>
              ))}
            </ul>
          ) : null
        }
      </Async>
      <Pager paged={paged} />
    </>
  );
}

export function ResearchProblemPage({ id, before }: { id: string; before?: string }) {
  const api = useApi();
  const load = useCallback(
    (signal: AbortSignal) => api.getResearchProblem(id, { signal }, before),
    [api, id, before],
  );
  const problem = useAsync(load);
  return (
    <div className="page">
      <Link to={{ kind: 'conjectures' }}>{t('Conjectures')}</Link>
      <Async state={problem.state} onRetry={problem.reload}>
        {(p) => (
          <article>
            <header className="page-head">
              <div>
                <h1>
                  <InlineLatex source={p.summary.title} />
                </h1>
                <p>{statusText(p.summary)}</p>
              </div>
            </header>
            {p.descriptions.map((d, index) => {
              const href = sourceHref(d.problem.source);
              return (
                <section key={`${d.id}-${index}`} aria-label={t('Original problem')}>
                  <h2>{t('Original problem')}</h2>
                  <LatexText source={d.statement} />
                  {d.proposed_by?.length ? <Posers people={d.proposed_by} /> : null}
                  <p>
                    {href ? (
                      <ExternalLink href={href}>{sourceLabel(d.problem.source)}</ExternalLink>
                    ) : (
                      sourceLabel(d.problem.source)
                    )}
                  </p>
                </section>
              );
            })}
            {p.curated ? (
              <CuratedEntry id={id} problem={p.curated} />
            ) : p.summary.source ? (
              <section aria-label={t('Original problem')}>
                <h2>{t('Original problem')}</h2>
                <p>
                  <SourceLink source={p.summary.source} />
                </p>
              </section>
            ) : null}
            {before ? (
              <p>
                <Link to={{ kind: 'research_problem', id }}>{t('Back to the first notes')}</Link>
              </p>
            ) : null}
            <section aria-labelledby="problem-notes-h">
              <h2 id="problem-notes-h">{t('Papers and notes')}</h2>
              {p.notes.map((note) => (
                <article key={note.record} className="paper-entry">
                  <h3>
                    <Link to={{ kind: 'paper', record: note.record }}>
                      <InlineLatex source={note.title} />
                    </Link>
                  </h3>
                  <AuthorLine authors={note.authors} />
                  <p className="entry-meta">
                    {t('Version ')}
                    {note.version} · <DateText iso={note.updated_at} /> · {note.license}
                  </p>
                  <p>
                    <Link to={{ kind: 'paper', record: note.record }}>
                      {t('Read full text and version history')}
                    </Link>
                    {' · '}
                    <ExternalLink href={note.source_url}>{t('Original publication')}</ExternalLink>
                  </p>
                  <ul>
                    {note.results.map((result) => (
                      <li key={result.claim}>
                        <Link to={{ kind: 'result', record: note.record, claim: result.claim }}>
                          <InlineLatex source={resultLabel({ ...result, id: result.claim })} />
                        </Link>
                        {result.outcome
                          ? ` · ${result.outcome === 'partial' ? 'Partial progress' : result.outcome === 'proved' ? 'Reported proof' : 'Reported refutation'}`
                          : ''}
                      </li>
                    ))}
                  </ul>
                  {note.evidence.filter((e) => e.kind === 'lean_source').length ? (
                    <p>
                      {t('Lean evidence:')}{' '}
                      {note.evidence
                        .filter((e) => e.kind === 'lean_source')
                        .map((e, i) => (
                          <span key={`${e.url}-${i}`}>
                            {i ? '; ' : ''}
                            <ExternalLink href={e.url}>{e.scope}</ExternalLink>
                          </span>
                        ))}
                    </p>
                  ) : null}
                  {note.contributions.length ? (
                    <details>
                      <summary>{t('Contributions recorded in this note')}</summary>
                      <ul>
                        {note.contributions.map((c) => (
                          <li key={c.id}>
                            <strong>{roles[c.role]}</strong> ·{' '}
                            {c.contributors.map((person) => person.name).join('; ')}
                            <LatexText source={c.scope} />
                            <ExternalLink href={c.evidence_url}>
                              {t('Contribution evidence')}
                            </ExternalLink>
                          </li>
                        ))}
                      </ul>
                    </details>
                  ) : null}
                </article>
              ))}
              {(p.collected_notes ?? []).map((note) => (
                <CollectedNote key={note.work} note={note} />
              ))}
              {p.curated && !before && !p.notes.length && !p.collected_notes?.length ? (
                <p>{t('No paper or note addresses this problem yet.')}</p>
              ) : null}
            </section>
            {p.next_before ? (
              <p>
                <Link to={{ kind: 'research_problem', id, before: p.next_before }}>
                  {t('More papers and notes')}
                </Link>
              </p>
            ) : null}
            <p>
              {t(
                'Follow a named result to read its statement, formal proofs and later contributions.',
              )}
            </p>
            <Link to={{ kind: 'formalizations' }}>{t('Contribute a formal proof')}</Link>
            {p.curated ? (
              <p className="entry-meta">
                {t('Statement text:')} {p.curated.license} · {t('Source licence:')}{' '}
                {p.curated.source_license} · {t('Version ')}
                {p.curated.version} · <DateText iso={p.curated.published_on} />
              </p>
            ) : null}
          </article>
        )}
      </Async>
    </div>
  );
}
