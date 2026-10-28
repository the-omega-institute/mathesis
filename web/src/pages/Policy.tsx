import { PROCESSING_CONSENT_TEXT } from '../api/types';
import { usePolicyState } from '../api/policy';
import { Async } from '../components/ui';
import { BASIS_DESCRIPTIONS, BASIS_LABELS, rejectReasonText } from '../lib/labels';
import { STAGES, THRESHOLD_TEXT, stageInfo, stageName } from '../lib/stages';
import type { AdmissionBasis, RejectReason } from '../api/types';

const BASES: AdmissionBasis[] = ['escape_witness', 'open_problem_settlement', 'open_conjecture'];

const REASONS: RejectReason[] = [
  { reason: 'hygiene', detail: 'the source does not compile or AI use is not disclosed' },
  { reason: 'known_result', claim: 'C1', prior: { kind: 'doi', locator: '10.…' } },
  { reason: 'bind_only' },
  { reason: 'no_main_result' },
  { reason: 'incorrect_result', claim: 'C1' },
  { reason: 'unchecked_result', claim: 'C1' },
  {
    reason: 'insufficient_evidence',
    claim: 'C1',
    detail:
      'The prerequisite comparison, literature search or claimed settlement needs more evidence.',
  },
  {
    reason: 'insufficient_utility',
    claim: 'C1',
    detail:
      'The computational result needs a concrete mathematical use and discharge of its exact prerequisites.',
  },
  {
    reason: 'conjecture',
    detail:
      'The conjecture needs a well-posed, open statement whose proof would carry new mathematical content.',
  },
  { reason: 'out_of_scope', detail: 'the paper is not a mathematics paper' },
];

export function PolicyPage() {
  const state = usePolicyState();
  return (
    <div className="page narrow-page">
      <section>
        <h2>Third-party AI review</h2>
        <p>{PROCESSING_CONSENT_TEXT}</p>
        <p>
          Uploads require explicit agreement. We store the consent version and time. Private review
          material stays private to authors and staff.
        </p>
      </section>
      <header className="page-head">
        <div>
          <h1>Review policy</h1>
          <p className="lede">
            Papers, short notes and conjectures receive AI review under the criteria below.
            Publication submissions receive an acceptance or refusal decision and one final report.
          </p>
        </div>
      </header>
      <section aria-labelledby="plain-h" className="notice notice-info">
        <h2 id="plain-h">In plain words</h2>
        <ol>
          <li>Upload a PDF or LaTeX project, check the manuscript PDF, and start the review.</li>
          <li>
            Mathesis reviews the arguments, checks findings against the manuscript and relevant
            prior work, and delivers one final report with analysis and useful suggestions.
          </li>
          <li>
            Work is accepted when a main result adds something the cited results do not already
            give, or settles a named open problem. Accepted reports explain the contribution and
            possible extensions; refused submissions receive the reasons for the decision.
          </li>
          <li>
            The final report appears on this site when it is ready. Review time varies with the
            manuscript and queue. Private reviews provide feedback without a publication decision.
          </li>
        </ol>
      </section>
      <Async state={state.status === 'idle' ? { status: 'loading' } : state}>
        {(policy) => (
          <>
            <section aria-labelledby="stages-h">
              <h2 id="stages-h">Stages</h2>
              <dl className="definitions">
                {STAGES.map((stage) => {
                  const info = stageInfo(stage, policy);
                  const human = policy.policy.human_judgement.includes(stage);
                  return (
                    <div key={stage}>
                      <dt>{stageName(stage, policy)}</dt>
                      <dd>{info.description}</dd>
                      <dd className="muted small">
                        {stage === 'claims'
                          ? 'Filed by the author’s confirmation.'
                          : human
                            ? 'Decided by an editor; machine reports on this stage are proposals.'
                            : 'Filed automatically.'}
                      </dd>
                    </div>
                  );
                })}
              </dl>
              <p className="small">
                An author may have at most {policy.policy.max_active_per_author} submissions in
                draft or review at once.
              </p>
            </section>
          </>
        )}
      </Async>

      <section aria-labelledby="threshold-h">
        <h2 id="threshold-h">Threshold</h2>
        <p>{THRESHOLD_TEXT} Short notes use the same threshold.</p>
        <p>
          An audited error, proof gap or unchecked main result blocks publication, even when another
          main result carries new content. Model review is not a proof of correctness. Confirmed
          error reports remain visible on the published version after acceptance.
        </p>
        <p>
          A conjecture is displayed when at least one main statement is audited as well posed, open,
          and mathematically new if proved.
        </p>
        <dl className="definitions">
          {BASES.map((b) => (
            <div key={b}>
              <dt>{BASIS_LABELS[b]}</dt>
              <dd>{BASIS_DESCRIPTIONS[b]}</dd>
            </div>
          ))}
        </dl>
      </section>

      <section aria-labelledby="escape-h">
        <h2 id="escape-h">Escape analysis: does the proof add new content?</h2>
        <p>
          A proved statement is <strong>bind-only</strong> when its proof obtains it from prior
          results by instantiation, substitution, projection, recombination or normalisation. It
          carries <strong>content</strong> when a live intermediate proposition or the conclusion
          itself requires a construction, computation or estimate beyond those operations. New
          aliases and unused lemmas do not establish content. Consumed bind-only helpers remain
          legitimate proof organization.
        </p>
        <p>
          New review rounds use Trureturing’s methodology to record the prerequisite baseline,
          actual search scope, new mathematical obligation, live witnesses and independent value.
          Correctness, proof escape, information gain and utility are separate judgments, with no
          combined score. Unavailable evidence remains undetermined and blocks automatic admission;
          it is not a finding that the mathematics has no value. Earlier rounds retain their
          original method.
        </p>
        <p>
          General theorems need no immediate application. Positive finite enumeration or an ordinary
          certified instance alone is supporting evidence. Checkers and numerical reductions need a
          concrete mathematical use and discharged prerequisites. A genuine counterexample answers
          its named assertion directly. Named open-problem settlements must match the inspected
          original statement and hypotheses, even when the proof is bind-only.
        </p>
        <p>
          Information-gain analysis requires an explicit model, readout, task and supporting
          evidence. It does not decide publication. No escape percentage is shown without a built
          and measured finite arena. Source-based review is not a compiled Trureturing registration
          or a Lean proof.
        </p>
      </section>

      <section aria-labelledby="reasons-h">
        <h2 id="reasons-h">Reasons a paper is not accepted</h2>
        <ul>
          {REASONS.map((r) => (
            <li key={r.reason}>{rejectReasonText(r)}</li>
          ))}
        </ul>
        <p>
          The report and its reasons stay private to the authors. The author may upload a revised
          version; it returns to draft and the statements are confirmed again.
        </p>
      </section>

      <section aria-labelledby="visibility-h">
        <h2 id="visibility-h">Visibility</h2>
        <ul>
          <li>
            A paper in draft, in review, not accepted or withdrawn is visible to its authors and the
            editors only.
          </li>
          <li>
            When the author allows volunteer contributors, signed-in contributors can read each
            statement, its dependencies, and the paper’s title and abstract while the work is in
            review.
          </li>
          <li>
            “Make public after acceptance” is checked by default for every kind. The public page
            shows statements, dependencies, audited witness propositions and verified Lean proofs.
            Authors can change the setting later; private records show only title, authors, kind and
            record id. Reviews, correctness labels, comments, reports, advice and letters stay
            private.
          </li>
        </ul>
        <p>
          Accepted conjectures receive an elaborated Lean statement after the letter. The author
          confirms its exact text or asks for a correction. This checks the statement’s meaning and
          well-formedness; it does not verify a proof. Attack attempts follow in Phase B.
        </p>
      </section>
    </div>
  );
}
