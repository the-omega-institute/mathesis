import { FeedbackWorkspace } from '../components/FeedbackWorkspace';
import { BindingWorkspace } from '../components/BindingWorkspace';
import { InlineLatex } from '../components/Markdown';
import { EvidenceInventory } from '../components/EvidenceFiles';
import { MyCma, CmaHistory } from '../components/MyCma';
import { useCallback, useEffect, useState } from 'react';
import { pdfHref, sourceFileHref } from '../api/client';
import { useApi } from '../api/context';
import { usePolicy } from '../api/policy';
import { useAsync } from '../api/useAsync';
import { useReferee } from '../api/useReferee';
import { authorReview } from '../lib/authorReview';
import { WorkspaceTabs, type WorkspaceSection } from '../components/WorkspaceTabs';
import type { Decision, PolicyDocument, RejectReason, Submission } from '../api/types';
import { hasRole, useSession } from '../auth/session';
import { AnalysisView, ConjectureList } from '../components/Analysis';
import { AuthorLine } from '../components/AuthorLine';
import { BasisBadge, SubmissionStatusBadge } from '../components/badges';
import { StatementList } from '../components/claims';
import { LatexText, MacrosProvider } from '../components/Markdown';
import { StageTimeline } from '../components/StageTimeline';
import { StatementSummary } from '../components/StatementSummary';
import { ReviewOverview } from '../components/ReviewOverview';
import { ResearchWorkspace } from '../components/ResearchWorkspace';
import { RefereeWorkspace } from '../components/referee';
import {
  Async,
  Badge,
  DateText,
  ExternalLink,
  Fold,
  Loading,
  SignInPrompt,
} from '../components/ui';
import {
  ContributorsToggle,
  FormalizationForAuthor,
  NewVersionForm,
  VisibilityChoice,
  LeanStatementCard,
  WithdrawPanel,
} from '../components/workspace/AuthorPanels';
import { ConfirmStatements } from '../components/workspace/ConfirmStatements';
import { EditorTools } from '../components/workspace/EditorTools';
import {
  AI_USE_LABELS,
  decisionHeadline,
  formatBytes,
  rejectReasonText,
  SUBMISSION_KIND_LABELS,
} from '../lib/labels';
import { sourceHref } from '../lib/links';
import { currentMacros } from '../lib/macros';
import { stageName } from '../lib/stages';
import { Link, useRouter } from '../routing/router';

export function SubmissionDetailPage({
  id,
  staffOnly = false,
}: {
  id: string;
  staffOnly?: boolean;
}) {
  const api = useApi();
  const { session, person } = useSession();
  const load = useCallback((signal: AbortSignal) => api.getSubmission(id, { signal }), [api, id]);
  const allowed = !staffOnly || hasRole(person, 'editor', 'reviewer', 'admin');
  const submission = useAsync(session.status === 'loading' || !allowed ? null : load);
  if (staffOnly && session.status === 'anonymous') {
    return (
      <div className="page">
        <SignInPrompt what="open the staff manuscript workspace" />
      </div>
    );
  }
  if (staffOnly && session.status !== 'loading' && !allowed) {
    return (
      <div className="page">
        <h1>Staff access required</h1>
        <p>This manuscript workspace is available to editors, reviewers and admins.</p>
      </div>
    );
  }
  return (
    <div className="page">
      {session.status === 'loading' ? <Loading /> : null}
      <Async state={submission.state} onRetry={submission.reload}>
        {(s) => (
          <Workspace
            key={s.id}
            submission={s}
            onChange={submission.replace}
            staffOnly={staffOnly}
            redirectStaff={!staffOnly}
          />
        )}
      </Async>
    </div>
  );
}

function isAuthorOf(s: Submission, personId: string | undefined): boolean {
  if (personId === undefined) return false;
  return s.submitter === personId || s.authors.some((a) => a.person === personId);
}

export function Workspace({
  submission: s,
  onChange,
  staffOnly = false,
  redirectStaff = false,
}: {
  submission: Submission;
  onChange: (s: Submission) => void;
  staffOnly?: boolean;
  redirectStaff?: boolean;
}) {
  const api = useApi();
  const policy = usePolicy();
  const { person } = useSession();
  const isAuthor = !staffOnly && isAuthorOf(s, person?.id);
  const staffPermission = hasRole(person, 'editor', 'reviewer', 'admin');
  const isStaff = staffOnly && staffPermission;
  const isEditor = isStaff && hasRole(person, 'editor', 'admin');
  const redirectToStaff = redirectStaff && !staffOnly && staffPermission && !isAuthor;
  const router = useRouter();
  const alreadyInStaffRoute = router.route.kind === 'staff_submission' && router.route.id === s.id;
  const state = s.status.state;
  const reviewed = state !== 'draft' && s.claims.length > 0;
  const referee = useReferee(s, isStaff, !redirectToStaff && (isStaff || isAuthor));
  useEffect(() => {
    if (!redirectToStaff || alreadyInStaffRoute) return;
    router.navigate({ kind: 'staff_submission', id: s.id }, { replace: true });
  }, [alreadyInStaffRoute, redirectToStaff, router, s.id]);
  const reviewFile = referee.value;
  useEffect(() => {
    if (!reviewFile) return;
    const controller = new AbortController();
    void api
      .getSubmission(s.id, { signal: controller.signal })
      .then((current) => {
        if (!controller.signal.aborted && current.revision > s.revision) onChange(current);
      })
      .catch(() => {
        /* The existing paper and referee fetches handle visible errors. */
      });
    return () => controller.abort();
  }, [api, s.id, s.revision, reviewFile, onChange]);
  useEffect(() => {
    if (s.status.state !== 'accepted' || !isAuthor) return;
    const controller = new AbortController();
    const refresh = () => {
      void api
        .getSubmission(s.id, { signal: controller.signal })
        .then((updated) => {
          if (!controller.signal.aborted && updated.revision > s.revision) onChange(updated);
        })
        .catch(() => {
          /* The existing view remains available during a temporary failure. */
        });
    };
    const pending =
      s.kind === 'conjecture' ||
      s.formalization.items.some(
        (item) => item.state.state === 'approved' || item.state.state === 'in_progress',
      );
    const timer = pending ? window.setInterval(refresh, 30_000) : undefined;
    window.addEventListener('focus', refresh);
    return () => {
      controller.abort();
      window.clearInterval(timer);
      window.removeEventListener('focus', refresh);
    };
  }, [api, s.id, s.kind, s.status.state, s.revision, s.formalization.items, isAuthor, onChange]);
  const [revising, setRevising] = useState(false);
  const showRevisionForm =
    revising &&
    (state === 'in_review' ||
      state === 'accepted' ||
      state === 'not_accepted' ||
      state === 'review_complete') &&
    isAuthor;

  // Keyed on the revision, so every change reloads the preview and the analysis.
  const revision = s.revision;
  const loadDecision = useCallback(
    (signal: AbortSignal) => {
      void revision;
      return api.previewDecision(s.id, { signal });
    },
    [api, s.id, revision],
  );
  const decision = useAsync(
    !isStaff || state === 'withdrawn' || s.purpose === 'review_only' ? null : loadDecision,
  );
  const loadAnalysis = useCallback(
    (signal: AbortSignal) => {
      void revision;
      return api.getAnalysis(s.id, { signal });
    },
    [api, s.id, revision],
  );
  const analysis = useAsync(isStaff && reviewed ? loadAnalysis : null);
  const preview = decision.value ?? s.decision ?? null;
  const current = s.versions[s.versions.length - 1];
  const review = authorReview(s, referee.value);
  const currentReview = review.current;
  const [selectedSection, setSection] = useState<WorkspaceSection | null>(null);
  const activeSection = selectedSection ?? (state === 'draft' ? 'manuscript' : 'review');
  const revise = () => {
    setRevising(true);
    setSection('manuscript');
  };

  if (redirectToStaff) {
    return (
      <p className="notice notice-info" role="status">
        Opening the staff manuscript workspace…
      </p>
    );
  }

  return (
    <MacrosProvider macros={currentMacros(s.versions)}>
      <article className={`workspace ${isStaff ? 'staff-workspace' : 'author-workspace'}`}>
        <header className="record-head">
          <h1>
            <InlineLatex source={s.title || 'Untitled paper'} />
          </h1>
          <AuthorLine authors={s.authors} />
          <p className="entry-meta">
            <span>
              {s.purpose === 'review_only' ? 'Private review' : SUBMISSION_KIND_LABELS[s.kind]}
            </span>
            {isStaff ? (
              <SubmissionStatusBadge status={s.status} />
            ) : (
              <Badge tone={review.status.tone}>{review.status.label}</Badge>
            )}
            <span>
              Version {current?.number ?? 1} · <DateText iso={s.created_at} />
            </span>
            {s.doi ? <DoiLink doi={s.doi} /> : null}
          </p>
          <div className="button-row">
            {current?.pdf ? (
              <a
                className="button"
                href={pdfHref(s.id, current.number)}
                target="_blank"
                rel="noopener noreferrer"
              >
                View PDF
              </a>
            ) : null}
            {current ? (
              <a className="button button-quiet" href={sourceFileHref(s.id, current.number)}>
                Download source
              </a>
            ) : null}
            {s.status.state === 'accepted' ? (
              <Link className="button button-quiet" to={{ kind: 'paper', record: s.status.record }}>
                Public page {s.status.record}
              </Link>
            ) : null}
          </div>
        </header>

        {staffPermission && isAuthor ? (
          <p className="workspace-switch">
            <Link to={{ kind: 'staff_submission', id: s.id }}>Open staff workspace</Link>
          </p>
        ) : isStaff && isAuthorOf(s, person?.id) ? (
          <p className="workspace-switch">
            <Link to={{ kind: 'submission', id: s.id }}>Open author workspace</Link>
          </p>
        ) : null}
        {isStaff ? (
          <ReviewOverview submission={s} round={currentReview} letters={referee.value?.letters} />
        ) : null}
        <WorkspaceTabs
          sections={
            isStaff
              ? ['review', 'history', 'manuscript', 'advanced']
              : ['manuscript', 'review', 'formalization']
          }
          active={activeSection}
          onChange={setSection}
        />
        <div
          className="workspace-panel"
          role="tabpanel"
          id="workspace-panel"
          aria-labelledby={`tab-${activeSection}`}
          tabIndex={0}
        >
          {activeSection === 'review' ? (
            <>
              {reviewed && state !== 'in_review' && isStaff ? (
                <StatementSummary submission={s} round={currentReview} />
              ) : null}
              {(state !== 'draft' || isStaff) && (isStaff || isAuthor) ? (
                <RefereeWorkspace
                  key={`${s.id}-${isStaff}`}
                  submission={s}
                  isStaff={isStaff}
                  isEditor={isEditor}
                  resource={referee}
                  onChange={onChange}
                  onRevise={isAuthor && !showRevisionForm ? revise : undefined}
                />
              ) : null}

              {isStaff ? (
                <div className="folds staff-folds">
                  {reviewed ? (
                    <Fold id="statements-h" title="Statements" hint={s.claims.length}>
                      <StatementList claims={s.claims} scope="ws" />
                    </Fold>
                  ) : null}

                  {reviewed ? (
                    <Fold id="analysis-h" title="Analysis">
                      <Async state={analysis.state} onRetry={analysis.reload}>
                        {(a) => <AnalysisView analysis={a} claims={s.claims} scope="ws" />}
                      </Async>
                      {state === 'accepted' ? (
                        <>
                          <h3>Conjecture follow-ups</h3>
                          <ConjectureList
                            conjectures={s.conjectures}
                            claims={s.claims}
                            scope="ws"
                          />
                        </>
                      ) : null}
                    </Fold>
                  ) : null}

                  {s.purpose !== 'review_only' ? (
                    <Fold id="stages-h" title="Publication decision">
                      {state === 'in_review' && preview ? (
                        <DecisionView decision={preview} policy={policy} />
                      ) : null}
                      <StageTimeline submission={s} policy={policy} decision={preview} />
                    </Fold>
                  ) : null}

                  {current ? (
                    <EvidenceInventory
                      files={current.evidence}
                      submission={s.id}
                      version={current.number}
                      showFingerprint
                    />
                  ) : null}
                </div>
              ) : null}

              {state === 'draft' && !isStaff ? (
                <p>Check the compiled PDF in Manuscript to start your AI review.</p>
              ) : null}
            </>
          ) : null}
          {activeSection === 'manuscript' ? (
            <>
              {showRevisionForm ? (
                <div id="standing-revision-form">
                  <NewVersionForm
                    submission={s}
                    onChange={(updated) => {
                      setRevising(false);
                      onChange(updated);
                    }}
                  />
                </div>
              ) : null}

              {state === 'draft' ? <DraftNotice submission={s} /> : null}
              {state === 'draft' && isAuthor ? (
                <section aria-labelledby="confirm-h">
                  <p className="eyebrow">Before review</p>
                  <h2 id="confirm-h">Check the compiled PDF</h2>
                  <p className="lede">
                    Open the compiled PDF above and make sure it matches your source. When it looks
                    right, start the review; Mathesis will identify the mathematical statements
                    automatically.
                  </p>
                  <ConfirmStatements
                    key={`${s.id}-${s.versions.length}`}
                    submission={s}
                    onChange={onChange}
                  />
                </section>
              ) : null}

              {isAuthor ? (
                <section className="submission-details" aria-labelledby="submission-details-h">
                  <div className="submission-details-heading">
                    <p className="eyebrow">Manuscript</p>
                    <h2 id="submission-details-h">Submission details</h2>
                  </div>
                  <div className="folds submission-detail-folds">
                    {current ? (
                      <EvidenceInventory
                        files={current.evidence}
                        submission={s.id}
                        version={current.number}
                        showFingerprint={false}
                      />
                    ) : null}
                    <Fold id="abstract-h" title="Abstract" open>
                      {s.abstract_text ? (
                        <LatexText source={s.abstract_text} className="abstract" />
                      ) : (
                        <p className="muted">No abstract was found in the source.</p>
                      )}
                      <p className="small">
                        <strong>AI disclosure:</strong> {AI_USE_LABELS[s.ai_disclosure.level]} —{' '}
                        {s.ai_disclosure.statement}
                      </p>
                    </Fold>

                    {state === 'accepted' ? (
                      <VisibilityChoice submission={s} onChange={onChange} />
                    ) : null}
                    <VersionsSection submission={s} showFingerprint={false} />

                    <Fold id="settings-h" title="Submission settings">
                      {s.purpose !== 'review_only' &&
                      state !== 'withdrawn' &&
                      state !== 'not_accepted' ? (
                        <ContributorsToggle submission={s} onChange={onChange} />
                      ) : null}
                      {state === 'draft' || (state === 'in_review' && !showRevisionForm) ? (
                        <NewVersionForm submission={s} onChange={onChange} />
                      ) : null}
                      {state === 'draft' || state === 'in_review' ? (
                        <WithdrawPanel submission={s} onChange={onChange} />
                      ) : null}
                    </Fold>
                  </div>
                </section>
              ) : (
                <div className="folds">
                  <Fold id="abstract-h" title="Abstract" open>
                    {s.abstract_text ? (
                      <LatexText source={s.abstract_text} className="abstract" />
                    ) : (
                      <p className="muted">No abstract was found in the source.</p>
                    )}
                    <p className="small">
                      <strong>AI disclosure:</strong> {AI_USE_LABELS[s.ai_disclosure.level]} —{' '}
                      {s.ai_disclosure.statement}
                    </p>
                  </Fold>

                  <VersionsSection submission={s} showFingerprint={isStaff} />
                </div>
              )}
            </>
          ) : null}
          {activeSection === 'formalization' && isAuthor ? (
            <section className="formalization-next" aria-labelledby="formalization-h">
              <h2 id="formalization-h">Formalization</h2>
              {state === 'accepted' ? (
                <>
                  <p>
                    Prepare precise Lean statements for accepted research, then develop and verify
                    their proofs.
                  </p>
                  {s.kind !== 'conjecture' ? (
                    <FormalizationForAuthor submission={s} onChange={onChange} />
                  ) : null}
                  {s.kind === 'conjecture' || s.lean_statements.length > 0 ? (
                    <LeanStatementCard submission={s} onChange={onChange} />
                  ) : null}
                </>
              ) : (
                <p className="muted">
                  {s.purpose === 'review_only'
                    ? 'This private review does not schedule formalization.'
                    : 'Formalization can be requested after acceptance. No formalization is scheduled for this manuscript.'}
                </p>
              )}
            </section>
          ) : null}
          {activeSection === 'history' && isStaff ? (
            <>
              <CmaHistory attempts={referee.value?.cma_attempts ?? []} />
              {!referee.value?.cma_attempts?.length ? (
                <p className="muted">No executions have been recorded.</p>
              ) : null}
              {s.submitter === person?.id ? (
                <Fold id="compute-h" title="Compute settings">
                  <MyCma />
                </Fold>
              ) : null}
            </>
          ) : null}
          {activeSection === 'advanced' && isStaff ? (
            <>
              <Fold id="binding-workspace-h" title="Binding discussions">
                <BindingWorkspace submission={s} />
              </Fold>
              <Fold id="feedback-workspace-h" title="Research feedback">
                <FeedbackWorkspace submission={s} />
              </Fold>
              <Fold id="research-workspace-h" title="Research follow-up">
                <ResearchWorkspace submission={s} />
              </Fold>
              {isEditor ? (
                <EditorTools
                  submission={s}
                  decision={preview}
                  person={person}
                  onChange={onChange}
                />
              ) : null}
            </>
          ) : null}
        </div>
      </article>
    </MacrosProvider>
  );
}

function DoiLink({ doi }: { doi: string }) {
  const href = sourceHref({ kind: 'doi', locator: doi });
  return href ? <ExternalLink href={href}>doi:{doi}</ExternalLink> : <span>doi:{doi}</span>;
}

function DraftNotice({ submission }: { submission: Submission }) {
  const current = submission.versions[submission.versions.length - 1];
  if (!current) return null;
  const warnings = current.parse_warnings;
  if (!current.compile_error && warnings.length === 0 && current.pdf) return null;
  return (
    <section aria-label="Reading the source" className="notices">
      {current.compile_error ? (
        <div className="notice notice-error" role="alert">
          <p>
            <strong>The PDF did not compile.</strong>
          </p>
          <pre className="compile-log">{current.compile_error}</pre>
        </div>
      ) : current.pdf ? null : (
        <p className="notice notice-info">The PDF is being compiled.</p>
      )}
      {warnings.length > 0 ? (
        <div className="notice notice-info">
          <p>
            <strong>
              Reading the source raised {warnings.length} warning{warnings.length === 1 ? '' : 's'}:
            </strong>
          </p>
          <ul>
            {warnings.map((w, i) => (
              <li key={i}>{w}</li>
            ))}
          </ul>
        </div>
      ) : null}
    </section>
  );
}

function DecisionView({ decision, policy }: { decision: Decision; policy: PolicyDocument | null }) {
  const headline = decisionHeadline(decision, (stage) => stageName(stage, policy));
  const tone =
    decision.decision === 'accept'
      ? 'decision-accept'
      : decision.decision === 'not_accepted'
        ? 'decision-reject'
        : 'decision-pending';
  return (
    <div className={`decision ${tone}`}>
      <p className="decision-head">
        <strong>{headline}</strong>
      </p>
      {decision.decision === 'pending' ? <p className="small">{decision.detail}</p> : null}
      {decision.decision === 'accept' ? (
        <p className="small">
          <BasisBadge basis={decision.basis} />
        </p>
      ) : null}
      {decision.decision === 'not_accepted' ? <ReasonList reasons={decision.reasons} /> : null}
    </div>
  );
}

export function ReasonList({ reasons }: { reasons: readonly RejectReason[] }) {
  return (
    <ul className="reason-list">
      {reasons.map((r, i) => (
        <li key={i}>{rejectReasonText(r)}</li>
      ))}
    </ul>
  );
}

function VersionsSection({
  submission,
  showFingerprint,
}: {
  submission: Submission;
  showFingerprint: boolean;
}) {
  return (
    <Fold id="versions-h" title="Versions" hint={submission.versions.length}>
      <ul className="version-list">
        {[...submission.versions].reverse().map((v) => (
          <li key={v.number}>
            <strong>Version {v.number}</strong> · <DateText iso={v.uploaded_at} withTime /> ·{' '}
            <code>{v.filename}</code> ({formatBytes(v.archive.bytes)}), main file{' '}
            <code>{v.main_file}</code>
            {v.pdf ? (
              <>
                {' '}
                · <a href={pdfHref(submission.id, v.number)}>PDF</a>
              </>
            ) : v.compile_error ? (
              <>
                {' '}
                <Badge tone="bad">PDF failed</Badge>
              </>
            ) : null}{' '}
            · <a href={sourceFileHref(submission.id, v.number)}>source</a>
            <EvidenceInventory
              files={v.evidence}
              submission={submission.id}
              version={v.number}
              showFingerprint={showFingerprint}
            />
            {v.note ? <span className="muted"> — {v.note}</span> : null}
          </li>
        ))}
      </ul>
    </Fold>
  );
}
