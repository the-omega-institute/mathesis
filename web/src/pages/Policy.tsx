import { usePolicyState } from '../api/policy';
import { Async } from '../components/ui';

export function PolicyPage() {
  const state = usePolicyState();
  return (
    <div className="page narrow-page">
      <header className="page-head">
        <div>
          <h1>Review policy</h1>
          <p className="lede">
            Mathesis reviews mathematical papers and conjectures with AI. Each submission receives
            one final report with the assessment, reasons and suggestions.
          </p>
        </div>
      </header>
      <section aria-labelledby="submission-h">
        <h2 id="submission-h">Submitting your work</h2>
        <p>
          Upload a PDF or LaTeX project, check the manuscript PDF, and confirm the title, abstract
          and authors. Submit when the manuscript is ready. Review time varies with the manuscript
          and the number of submissions waiting.
        </p>
        <Async state={state.status === 'idle' ? { status: 'loading' } : state}>
          {(policy) => (
            <p>
              You may have up to {policy.policy.max_active_per_author} submissions in preparation or
              review at a time.
            </p>
          )}
        </Async>
      </section>
      <section aria-labelledby="criteria-h">
        <h2 id="criteria-h">What we look for</h2>
        <p>
          A paper is accepted when its main results are supported by complete arguments and at least
          one adds substantive mathematical content or settles a named open problem. We compare the
          results with relevant prior work and examine the proofs and their assumptions. Papers and
          short notes use the same criteria.
        </p>
        <p>
          A direct consequence or reformulation of known results does not normally qualify as a new
          contribution. Settling a previously open problem can qualify even when the argument uses
          established methods. General theorems do not need an immediate application. Computational
          results need a clear mathematical purpose, justified assumptions and supporting evidence.
        </p>
        <p>
          A conjecture may be accepted for display when it is clearly stated, remains open on the
          evidence available, and its resolution would add mathematical knowledge. Acceptance does
          not claim that the conjecture is true.
        </p>
      </section>
      <section aria-labelledby="decision-h">
        <h2 id="decision-h">Your decision and report</h2>
        <p>
          Publication submissions receive an acceptance or rejection decision. An accepted report
          explains the contribution and possible extensions. A rejection explains the reasons, such
          as an error or gap in a main proof, insufficient evidence to assess a main result, or a
          result already contained in prior work. An unresolved main result prevents acceptance even
          when other results have merit.
        </p>
        <p>
          If a review cannot be completed, we say so. An interrupted review is not a mathematical
          rejection. Once the review has finished, you may respond to the report or submit a revised
          manuscript.
        </p>
      </section>
      <section aria-labelledby="privacy-h">
        <h2 id="privacy-h">Publication and privacy</h2>
        <p>
          “Make my results public once accepted” is selected by default. Accepted work is displayed
          with its authors and mathematical results. Uncheck this option if you want only a private
          review; that review provides feedback without a publication decision or public record.
        </p>
        <p>
          Drafts and review reports are private to the authors and Mathesis staff. Review reports
          remain private after acceptance. Authors may separately choose to share statements with
          contributors or change the public visibility of their mathematical details.
        </p>
        <p>
          Manuscripts and supporting material are processed by external AI systems for review.
          Uploading prepares a draft; review begins when you submit it.
        </p>
      </section>
      <section aria-labelledby="formalization-h">
        <h2 id="formalization-h">Formalization and corrections</h2>
        <p>
          AI review does not constitute a formal proof. For suitable accepted work, Mathesis may
          explore a Lean formalization. The author confirms that the formal statement matches the
          intended mathematics before community proof contributions begin. Formal proofs are
          published with the author’s approval and credit to their contributors.
        </p>
        <p>
          A verified Lean result proves or disproves the confirmed formal statement. Readers can
          inspect the proof source and report errors in a published result or its formalization.
          Confirmed corrections remain associated with the published work.
        </p>
      </section>
    </div>
  );
}
