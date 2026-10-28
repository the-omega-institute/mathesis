/*
 * Wire types for the wishpool HTTP API v3, copied from `docs/API.md`.
 * Keep this file in step with that document; the frontend calls no route
 * the document does not list.
 */

// ── Errors and listings ──────────────────────────────────────────────────

export type ProblemCode =
  | 'not_authenticated'
  | 'forbidden'
  | 'not_found'
  | 'invalid'
  | 'conflict'
  | 'stale_revision'
  | 'unavailable';

/** An `application/problem+json` body, read defensively. */
export interface Problem {
  type?: string;
  title?: string;
  status?: number;
  detail?: string;
  code?: string;
}

export type Listing<T> = { items: T[]; next_before: string | null };

// ── Referee rounds and editorial feedback ────────────────────────────────

export type Recommendation = 'accept' | 'minor_revision' | 'major_revision' | 'reject';
export type Severity = 'major' | 'minor';
export type Correctness = 'correct' | 'gap' | 'error' | 'not_checked';
export type AuditedClaim = {
  coverage?: ClaimCoverage;
  methodology?: MethodologyReading;
  conjecture?: ConjectureReading;
  claim: string;
  correctness: Correctness;
  comment: string;
  shape?: ProofShape | null;
  witnesses: string[];
  known?: string | null;
  referee_agreed: boolean;
};
export type RefereeAudit = {
  response_assessments?: {
    response_id: string;
    status: 'addressed' | 'unresolved' | 'not_checkable';
    explanation: string;
    evidence: { path: string; sha256: string }[];
  }[];
  verdict: Recommendation;
  agrees_with_referee: boolean;
  summary: string;
  claims: AuditedClaim[];
  concerns: {
    concern: string;
    status: 'confirmed' | 'refuted' | 'not_checkable';
    note: string;
    evidence?: string;
    basis?: string;
    location?: string;
  }[];
};

export type ImprovementKind =
  'gap' | 'strengthen' | 'generalize' | 'computation' | 'literature' | 'exposition';
export type Effort = 'small' | 'medium' | 'large';
export type ImprovementEvidence = 'checked' | 'proposed';
export type Feasibility = 'ready' | 'needs_library' | 'hard';

export type RefereeConcern = { claim?: string; severity: Severity; issue: string };
export type RefereeClaim = {
  conjecture?: ConjectureReading;
  claim: string;
  shape: ProofShape;
  witnesses: string[];
  known?: string;
  note: string;
};
export type RefereeReport = {
  recommendation?: Recommendation;
  summary: string;
  strengths: string[];
  concerns: RefereeConcern[];
  claims: RefereeClaim[];
  limits: string[];
  text: string;
};
export type Improvement = {
  claim?: string;
  kind: ImprovementKind;
  suggestion: string;
  how_we_help: string;
  effort: Effort;
  status: ImprovementEvidence;
  evidence: string;
};
export type FormalizationCandidate = {
  claim: string;
  feasibility: Feasibility;
  mathlib: string[];
  missing: string[];
  lean_sketch: string;
  plan: string;
  effort: Effort;
};
export type Advice = {
  summary: string;
  improvements: Improvement[];
  formalization: FormalizationCandidate[];
};
export type LetterDraft = { subject: string; body: string; note: string };
export type ProbeOutcome = 'compiled' | 'failed';
export type FormalAttempt = {
  claim: string;
  outcome: ProbeOutcome;
  theorem?: string;
  lean?: string;
  axioms?: string[];
  note?: string;
  log?: string;
};
/** A private, staff-only formalization probe; never a recorded formalization. */
export type FormalProbe = { toolchain?: string; attempts: FormalAttempt[]; summary?: string };

export type StepState<T> =
  | { state: 'pending' }
  | { state: 'running'; task?: string; queue_position?: number; since: string }
  | { state: 'done'; result: T; at: string }
  | { state: 'failed'; reason: string; detail?: string; retryable: boolean; at: string }
  | { state: 'skipped'; reason: string };
export type Step<T> = {
  engine?: string;
  model?: string;
  attempts: number;
  client_ref?: string;
  input_digest?: string;
  state: StepState<T>;
};
export type RefereeRound = {
  computation_contract?: number;
  computation_receipts?: ComputationReceipt[];
  knowledge_contract?: number;
  knowledge_events?: KnowledgeEvent[];
  review_contract?: number;
  review_method?: string;
  /** Safe lifecycle projection; older servers may omit it. */
  progress?: Record<'referee' | 'audit' | 'advice' | 'letter' | 'formal', StageProgress>;
  source_sha256?: string;
  response_ids?: string[];
  number: number;
  version: number;
  claims_revision: number;
  started_at: string;
  referee: Step<RefereeReport>;
  audit?: Step<RefereeAudit>;
  /** Omitted from the author projection. */
  advice?: Step<Advice>;
  /** Absent on rounds stored before the probe existed. */
  formal?: Step<FormalProbe>;
  /** Drafts are staff-only; authors read sent letters. */
  letter?: Step<LetterDraft>;
};
export type StageProgress =
  | { state: 'pending' | 'skipped' }
  | { state: 'running'; since: string }
  | { state: 'done'; at: string }
  | { state: 'failed'; at: string; retryable: boolean };
export type FeedbackLetter = {
  round?: number;
  assessment?: Recommendation | null;
  subject: string;
  body: string;
  note: string;
  edited: boolean;
  sent_by: string;
  sent_at: string;
};
export type FindingResponse = {
  id: string;
  round: number;
  concern: number;
  version: number;
  claims_revision: number;
  body: string;
  author: string;
  at: string;
};
export type NewFindingResponse = Pick<
  FindingResponse,
  'round' | 'concern' | 'version' | 'claims_revision' | 'body'
>;
export type RefereeFile = {
  finding_responses?: FindingResponse[];
  cma_attempts?: CmaAttemptView[];
  id: string;
  rounds: RefereeRound[];
  letters: FeedbackLetter[];
  revision: number;
};
export type SendFeedback = {
  subject: string;
  body: string;
  note?: string;
  assessment?: Recommendation;
};

// ── People and sign-in ───────────────────────────────────────────────────

export type Role = 'editor' | 'endorser' | 'reviewer' | 'admin';
export type Person = {
  id: string;
  display_name: string;
  email?: string;
  picture?: string;
  orcid?: string;
  affiliation?: string;
  roles: Role[];
  created_at: string;
  last_seen_at: string;
};

/** `GET /auth/session`. */
export type Session =
  | { authenticated: false; donations_enabled?: boolean; dev_sign_in?: boolean }
  | { authenticated: true; person: Person; donations_enabled?: boolean; dev_sign_in?: boolean };

// ── Policy ───────────────────────────────────────────────────────────────

export type Stage = 'hygiene' | 'claims' | 'literature' | 'escape';
/** Stages an editor or reviewer account files reports on (`claims` is the author's). */
export type ReportedStage = Exclude<Stage, 'claims'>;

export type Policy = {
  human_judgement: Stage[];
  required_endorsements: number;
  max_active_per_author: number;
};
export type PolicyStage = { stage: Stage; code: string; title: string; description: string };
/** `GET /policy`. */
export type PolicyDocument = { policy: Policy; stages: PolicyStage[] };

// ── Papers ───────────────────────────────────────────────────────────────

export type AiUseLevel = 'none' | 'assisted' | 'substantial' | 'primarily';
export type AiDisclosure = { level: AiUseLevel; statement: string };

export type SubmissionKind = 'paper' | 'note' | 'conjecture';
export type TypedConjecture = {
  title: string;
  statement: string;
  background?: string;
  origin?: string;
};
export type NewPaper = {
  title?: string;
  abstract_text?: string;
  purpose?: 'publication' | 'review_only';
  review_scope?: string;
  processing_consent?: string;
  kind?: SubmissionKind;
  make_public_after_acceptance?: boolean;
  typed_conjecture?: TypedConjecture;
  ai_disclosure: AiDisclosure;
  /** Overrides `\author`. */
  authors?: Author[];
  /** e.g. "11B83". */
  msc?: string[];
  /** The paper's persistent identifier, when it already has one. */
  doi?: string | null;
  open_to_contributors?: boolean;
};
export type Author = {
  name: string;
  person?: string;
  orcid?: string;
  affiliation?: string;
  google_scholar?: string;
  website?: string;
};

export type SourceKind =
  'doi' | 'arxiv' | 'hexagon' | 'zenodo' | 'oeis' | 'url' | 'personal' | 'named_work';
export type Source = { kind: SourceKind; locator: string; year?: number };

export type ClaimKind =
  'theorem' | 'proposition' | 'lemma' | 'corollary' | 'claim' | 'conjecture' | 'question';
export type ClaimRole = 'main' | 'supporting';
export type Settles = { name: string; source: Source };
export type Claim = {
  /** C1, C2, … */
  id: string;
  kind: ClaimKind;
  label: string;
  latex_label?: string;
  /** LaTeX. */
  statement: string;
  role: ClaimRole;
  has_proof: boolean;
  section?: string;
  depends_on: string[];
  settles?: Settles;
};
export type ClaimConfirmation = {
  depends_on_conjectures?: { record: string; claim: string }[];
  id: string;
  kind: ClaimKind;
  role: ClaimRole;
  depends_on?: string[];
  settles?: Settles | null;
  excluded?: boolean;
};

export type Blob = { id: string; bytes: number; sha256: string };
export type PaperVersion = {
  evidence?: EvidenceAttachment[];
  number: number;
  archive: Blob;
  filename: string;
  main_file: string;
  pdf?: Blob;
  compile_error?: string;
  parse_warnings: string[];
  /** Preamble math macros, KaTeX syntax (`"\\rep": "\\operatorname{rep}"`). */
  macros: Record<string, string>;
  note: string;
  uploaded_at: string;
};

export type SubmissionStatus =
  | { state: 'draft' }
  | { state: 'in_review' }
  | { state: 'review_complete' }
  | { state: 'accepted'; record: string }
  | { state: 'not_accepted' }
  | { state: 'withdrawn' };
export type AnalysisVisibility = 'undecided' | 'public' | 'private';

export type Submission = {
  purpose?: 'publication' | 'review_only';
  review_scope?: string;
  conjecture_dependencies?: {
    from: string;
    target: { record: string; claim: string };
    target_version: number;
    target_claims_revision: number;
  }[];
  kind: SubmissionKind;
  lean_statements: LeanStatementAttempt[];
  id: string;
  submitter: string;
  title: string;
  abstract_text: string;
  authors: Author[];
  ai_disclosure: AiDisclosure;
  msc: string[];
  doi?: string;
  /** The last is current. */
  versions: PaperVersion[];
  /** Read from the current version, awaiting confirmation. */
  extracted: Claim[];
  /** Confirmed. */
  claims: Claim[];
  claims_revision: number;
  /** Append-only; the latest per stage for the current claims counts. */
  reports: StageReport[];
  status: SubmissionStatus;
  decision?: Decision;
  open_to_contributors: boolean;
  analysis_visibility: AnalysisVisibility;
  formalization: { repository?: string; items: FormalizationItem[] };
  conjectures: ConjectureFollowUp[];
  created_at: string;
  updated_at: string;
  revision: number;
};

// ── Review ───────────────────────────────────────────────────────────────

export type Evidence = { kind: string; locator: string; note: string };
export type Ratio = { numerator: number; denominator: number };

export type RejectReason =
  | { reason: 'hygiene'; detail: string }
  | { reason: 'known_result'; claim: string; prior: Source }
  | { reason: 'bind_only' }
  | { reason: 'no_main_result' }
  | { reason: 'incorrect_result'; claim: string }
  | { reason: 'unchecked_result'; claim: string }
  | { reason: 'insufficient_evidence' | 'insufficient_utility'; claim: string; detail: string }
  | { reason: 'conjecture'; detail: string }
  | { reason: 'out_of_scope'; detail: string };

export type Outcome =
  | { outcome: 'pass' }
  | { outcome: 'fail'; reason: RejectReason }
  | { outcome: 'needs_human'; question: string };

export type Reviewer =
  | { kind: 'human'; person: string }
  | { kind: 'machine'; account: string; engine: string; model?: string };

export type HygieneCheck = { name: string; passed: boolean; detail: string };
export type PriorRelation = 'same' | 'implies' | 'related';
export type PriorWork = { claim: string; source: Source; relation: PriorRelation; note: string };
export type ProofShape = 'bind_only' | 'content' | 'undetermined';
export type EvidenceStatus = 'supported' | 'unsupported' | 'insufficient';
export type MethodologyReading = {
  knowledge?: {
    receipt_ids: string[];
    complete: boolean;
    audited_relation: CoverageRelation | null;
  };
  baseline: { statement: string; location: string; source: string | null }[];
  searched: string[];
  literature: EvidenceStatus;
  binding_analysis: string;
  delta: string;
  escape: ProofShape;
  witnesses: {
    kind: 'intermediate' | 'conclusion';
    statement: string;
    location: string;
    non_binding_step: string;
    live_use: string;
  }[];
  utility: {
    kind:
      | 'general_theorem'
      | 'finite_enumeration'
      | 'checker'
      | 'numeric_reduction'
      | 'certified_instance'
      | 'counterexample';
    status: EvidenceStatus;
    rationale: string;
    problem: string;
    progress: string;
    prerequisites: string;
    discharge: string;
  };
  settlement: {
    status: EvidenceStatus;
    problem: Settles;
    statement: string;
    assumptions: string;
    prior_status: string;
    resolution: string;
    matching_argument: string;
    opened_source: string;
  } | null;
  information_gain: {
    model: string;
    readout: string;
    task: string;
    correspondence: string;
    evidence: string;
    status: EvidenceStatus;
  } | null;
  limits: string[];
};
export type EscapeRate = { arena: string; before: Ratio; after: Ratio; artifact: string };
export type EscapeAssessment = {
  methodology?: MethodologyReading;
  conjecture?: ConjectureReading;
  claim: string;
  shape: ProofShape;
  witnesses: string[];
  rationale: string;
  correctness?: Correctness;
  escape_rate?: EscapeRate;
};

export type StagePayload =
  | { stage: 'hygiene'; checks: HygieneCheck[] }
  | { stage: 'claims'; claims: Claim[] }
  | { stage: 'literature'; prior: PriorWork[]; searched: string[] }
  | { stage: 'escape'; assessments: EscapeAssessment[] };

export type StageReport = {
  stage: Stage;
  outcome: Outcome;
  summary: string;
  payload: StagePayload;
  evidence: Evidence[];
  reviewer: Reviewer;
  claims_revision: number;
  filed_at: string;
};
export type ReportDraft = {
  outcome: Outcome;
  summary: string;
  payload: StagePayload;
  evidence?: Evidence[];
};
/** `POST /submissions/{id}/stages/{stage}/reports`. */
export type FileReport = { report: ReportDraft; filed_by?: { engine: string; model?: string } };

export type AdmissionBasis = 'escape_witness' | 'open_problem_settlement' | 'open_conjecture';
export type Decision =
  | { decision: 'pending'; awaiting: Stage; detail: string }
  | { decision: 'accept'; basis: AdmissionBasis }
  | { decision: 'not_accepted'; reasons: RejectReason[] };

export type NewJudgement = { shape: ProofShape; witnesses: string[]; rationale: string };
export type ClaimJudgement = {
  id: string;
  submission: string;
  claim: string;
  claims_revision: number;
  shape: ProofShape;
  witnesses: string[];
  rationale: string;
  reviewer: Reviewer;
  task?: string;
  filed_at: string;
};

export type JudgementStanding = 'confirmed' | 'corroborated' | 'proposed' | 'disputed';
export type ClaimAnalysis = {
  claim: string;
  kind: ClaimKind;
  role: ClaimRole;
  label: string;
  prior: PriorWork[];
  known: boolean;
  assessment?: EscapeAssessment;
  judgement?: {
    shape?: ProofShape;
    standing: JudgementStanding;
    witnesses: string[];
    judgements: number;
  };
  formalization?: ItemState;
};
export type PaperAnalysis = {
  submission: string;
  main_results: number;
  main_with_content: number;
  main_known: number;
  open_statements: number;
  formalized: number;
  claims: ClaimAnalysis[];
};

// ── After acceptance ─────────────────────────────────────────────────────

export type FormalArtifact = {
  repository: string;
  /** 40 hex. */
  commit: string;
  declarations: string[];
};
export type ItemState =
  | { state: 'proposed' }
  | { state: 'approved' }
  | { state: 'declined'; reason: string }
  | { state: 'in_progress' }
  | { state: 'verified'; artifact: FormalArtifact; axioms: string[]; contribution?: string };
export type FormalizationItem = {
  claim: string;
  reason: string;
  state: ItemState;
  updated_at: string;
};
export type FormalVerification = {
  artifact: FormalArtifact;
  axioms: string[];
  contribution?: string;
};

export type SettlementOutcome = 'proved' | 'disproved' | 'partial';
export type ConjectureState =
  | { state: 'screening' }
  | { state: 'taken_up' }
  | { state: 'not_pursued'; reason: string }
  | { state: 'settled'; outcome: SettlementOutcome; summary: string; evidence: Evidence[] };
export type ConjectureFollowUp = { claim: string; state: ConjectureState; updated_at: string };

export type ConjectureReading = {
  well_posed: boolean;
  well_posed_reason: string;
  status: 'open' | 'known_true' | 'known_false' | 'special_case_of_known' | 'unclear';
  status_reason: string;
  named_works: string[];
  escape: ProofShape;
  escape_reason: string;
  suggestions: string[];
};
export type LeanStatementAttempt = {
  faithfulness?: {
    verdict: 'faithful' | 'weaker' | 'stronger' | 'different' | 'trivial';
    reason: string;
    mismatch: string;
    regenerated: boolean;
  } | null;
  claim: string;
  version: number;
  claims_revision: number;
  lean: string;
  digest: string;
  toolchain: string;
  reading: string;
  created_at: string;
  response:
    | { state: 'awaiting_author' }
    | { state: 'confirmed'; author: string; at: string }
    | { state: 'rejected'; comment: string; at: string };
};
export type ConjectureSummary = {
  kind?: SubmissionKind;
  authors?: Author[];
  accepted_at?: string;
  new_results?: number;
  lean_verified?: number;
  claim: string;
  source: string;
  status: 'open' | 'solved' | 'disproved';
  attempts: number;
  solver: Entrant | null;
  record: string;
  title: string;
  statement: string;
  lean_statement_status: 'none' | 'awaiting_author' | 'confirmed';
};

// ── Public papers ────────────────────────────────────────────────────────

export type PaperSummary = {
  record: string;
  kind: SubmissionKind;
  title: string;
  authors: Author[];
  submission?: string;
  abstract_text?: string;
  msc?: string[];
  doi?: string;
  basis?: AdmissionBasis;
  accepted_at?: string;
  main_results?: number;
  new_results?: number;
  lean_verified?: number;
};
export type PublicClaim = {
  verified?: VerifiedAttempt | null;
  id: string;
  kind: ClaimKind;
  role: ClaimRole;
  label: string;
  statement: string;
  section?: string;
  depends_on: string[];
  lean?: FormalArtifact | null;
};
export type PublicPaper = {
  summary: PaperSummary;
  confirmed_error_reports?: number;
  versions?: { number: number; uploaded_at: string; has_pdf: boolean }[];
  claims?: PublicClaim[];
  formalization_repository?: string;
  macros?: Record<string, string>;
  new_content?: { claim: string; lemmas: string[] }[];
  lean_statements?: { claim: string; lean: string; digest: string; toolchain: string }[];
};

// ── Contributors ─────────────────────────────────────────────────────────

export type TaskKind = 'judge_escape' | 'literature_check' | 'formalize' | 'probe';
export type ContributionMode = 'own_agent' | 'hosted';
export type TaskStatus =
  | { state: 'open' }
  | { state: 'leased'; lease: { holder: string; mode: ContributionMode; until: string } }
  | { state: 'submitted'; contribution: string }
  | { state: 'done'; contribution: string }
  | { state: 'closed'; reason: string };
export type TaskState = TaskStatus['state'];
export type Task = {
  id: string;
  kind: TaskKind;
  target: { submission: string; claim: string };
  dedupe_key: string;
  title: string;
  contributors: string[];
  status: TaskStatus;
  created_by: string;
  created_at: string;
  updated_at: string;
  revision: number;
};
export type TaskContext = {
  task: Task;
  paper_title: string;
  abstract_text: string;
  claim: Claim;
  dependencies: Claim[];
  repository?: string;
  pdf_public: boolean;
  nature: string;
  macros: Record<string, string>;
};
/** `POST /submissions/{id}/tasks`. */
export type TaskGeneration = { created: number; existing: number };

export type ContributionOutput =
  | { output: 'judgement'; shape: ProofShape; witnesses: string[]; rationale: string }
  | { output: 'literature'; prior: PriorWork[]; searched: string[]; summary: string }
  | { output: 'probe_note'; note: string }
  | { output: 'pull_request'; url: string };
export type NewContribution = {
  agent: { tool: string; model: string };
  output: ContributionOutput;
  tokens?: { input: number; output: number };
};
export type ContributionStatus =
  | { state: 'submitted' }
  | { state: 'verified'; detail: string }
  | { state: 'rejected'; reason: string };
export type Contribution = {
  id: string;
  task: string;
  kind: TaskKind;
  contributor: string;
  mode: ContributionMode;
  agent: { tool: string; model: string };
  output: ContributionOutput;
  tokens?: { input: number; output: number; metered: boolean };
  status: ContributionStatus;
  submitted_at: string;
  updated_at: string;
  revision: number;
};
/** `POST /tasks/{id}/contributions`. */
export type SubmittedContribution = { contribution: Contribution; nature: unknown };
export type ContributionReview = { accept: boolean; note: string };

export type Credit = {
  contributor: string;
  verified: number;
  submitted: number;
  rejected: number;
  verified_formalizations: number;
  metered_tokens: number;
  reported_tokens: number;
};

export type GrantStatus = 'active' | 'paused' | 'revoked';
export type DonationGrant = {
  donor: string;
  monthly_cap: number;
  model: string;
  period: string;
  used: number;
  status: GrantStatus;
  created_at: string;
  updated_at: string;
  revision: number;
};
export type DonationPatch = { monthly_cap?: number; status?: GrantStatus };

export type Entrant = {
  id: string;
  name: string;
  kind: 'person' | 'agent';
  owner?: { id: string; name: string };
  retired: boolean;
  revision: number;
};
export type VerificationReceipt = {
  verdict: 'proved' | 'disproved' | 'rejected';
  reason: string;
  target_digest: string;
  solution_digest: string;
  toolchain: string;
  axioms: string[];
  checked_at: string;
  duration: number;
};
export type VerifiedAttempt = {
  modules?: Record<string, string>;
  attribution?: FormalizationAttribution;
  id: string;
  entrant: Entrant;
  receipt: VerificationReceipt;
  solution: string;
  also_verified: boolean;
};
export type AttemptView = {
  modules?: Record<string, string>;
  attribution?: FormalizationAttribution;
  id: string;
  record: string;
  claim: string;
  entrant: Entrant;
  state: 'queued' | 'proved' | 'disproved' | 'rejected' | 'superseded';
  reason?: string | null;
  receipt: VerificationReceipt | null;
  solution: string;
  note?: string;
};
export type ConjectureDetail = {
  summary: ConjectureSummary;
  macros: Record<string, string>;
  target: { claim: string; lean: string; digest: string; toolchain: string } | null;
  verified_attempts: VerifiedAttempt[];
};
export type LeaderboardRow = {
  rank: number;
  entrant: Entrant;
  solved: number;
  disproved: number;
  score: number;
  others: number;
  total: number;
  last_solve: string;
};
export type EntrantProfile = {
  others: number;
  total: number;
  errors_found: number;
  formalized: {
    record: string;
    claim: string;
    title: string;
    attempt: string;
    receipt: VerificationReceipt;
  }[];
  entrant: Entrant;
  solutions: {
    record: string;
    claim: string;
    title: string;
    attempt: string;
    receipt: VerificationReceipt;
  }[];
};
export type SolveNotification = {
  attempt: string;
  record: string;
  claim: string;
  entrant: Entrant;
  at: string;
};

export type CmaBinding = {
  person: string;
  subject: string;
  revision: number;
  generation: number;
  status: 'disconnected' | 'setup_needed' | 'ready' | 'unusable';
  reason: string;
  workspace_id: string | null;
  sandbox_id: string | null;
  sandbox_profile: { id: string; revision: number } | null;
  checked_at: string | null;
  created_at: string;
  updated_at: string;
};
export type MyCmaStatus = { enabled: boolean; binding: CmaBinding; setup_url: string };
export type CmaAttemptView = {
  kind?:
    | 'audit'
    | 'advice'
    | 'letter'
    | 'lean_probe'
    | 'conjecture_target'
    | 'research_target'
    | 'research_faithfulness'
    | 'problem_audit'
    | 'target_faithfulness'
    | 'error_report_audit'
    | 'binding_audit'
    | 'venue_attempt'
    | 'legacy_unknown';
  round?: number | null;
  claim?: string | null;
  target_generation?: number | null;
  step: string;
  attempt: number;
  payer: 'author' | 'operator';
  reason: string;
  status: 'running' | 'succeeded' | 'failed';
  admitted_at: string | null;
  finished_at: string | null;
};

export const PROCESSING_CONSENT_VERSION = 'ai-review-2026-10-11';
export const PROCESSING_CONSENT_TEXT =
  'I agree that the source and statements may be processed by external AI systems for mathematical review.';
export type ErrorSubject =
  | { kind: 'statement'; claim: string }
  | { kind: 'target'; claim: string; digest: string }
  | { kind: 'review' };
export type NewErrorReport = {
  subject: ErrorSubject;
  text: string;
  solution?: string;
  as_agent?: string;
};
export type ErrorReport = {
  id: string;
  record: string;
  reporter: Entrant;
  subject: ErrorSubject;
  text: string;
  created_at: string;
  resolution: {
    verdict: 'confirmed' | 'refuted' | 'unclear';
    reason: string;
    evidence: string;
    at: string;
  } | null;
  solution?: string;
  receipt?: VerificationReceipt;
};
export type Supporter = { name: string; tokens: number };
export type StatementDetail = {
  record: string;
  title: string;
  claim: PublicClaim;
  target: ConjectureDetail['target'];
  verified_attempts: VerifiedAttempt[];
  macros: Record<string, string>;
};

export interface NewLeanDeposit {
  project_files?: Record<string, string>;
  modules?: Record<string, string>;
  formalizers?: string[];
  source_revision?: string;
  title: string;
  authors: string[];
  statement: string;
  source_reference: Source;
  toolchain: string;
  license: string;
  solution: string;
}
export interface LeanDeposit extends Omit<NewLeanDeposit, 'solution' | 'modules'> {
  id: string;
  owner: string;
  modules?: Record<string, { id: string; bytes: number; sha256: string }>;
  source: { id: string; bytes: number; sha256: string };
  submitted_at: string;
  attempts: string[];
  revision: number;
}

export interface FormalizationAttribution {
  formalizers?: string[];
  source_revision?: string;
  deposit: string;
  authors: string[];
  source_reference: Source;
  license: string;
}

export interface FormalizationOpportunity {
  record: string;
  claim: string;
  title: string;
  label: string;
  statement: string;
  authors: Author[];
  kind: 'paper' | 'conjecture';
  status: 'open' | 'verified' | 'disproved';
  target_digest: string;
  toolchain: string;
  verified_count: number;
  contributor: Entrant | null;
  attribution: FormalizationAttribution | null;
  receipt: VerificationReceipt | null;
}

export interface LeanPreflight {
  modules: number;
  source_bytes: number;
  errors: string[];
  warnings: string[];
  dependencies: string[];
  requires_adapter: boolean;
}
export interface EvidenceAttachment {
  replay_protocol?: string;
  path: string;
  kind: 'certificate' | 'computation' | 'source' | 'data' | 'supplement';
  description: string;
  claims: string[];
  source_url?: string;
  source_revision?: string;
  bytes: number;
  sha256: string;
}

// Private knowledge evidence; a query hit is a candidate, not an entailment proof.
export type CoverageRelation = 'unknown' | 'candidate' | 'covered' | 'refuted';
export type CoverageSubject = {
  submission: string;
  version: number;
  claims_revision: number;
  round: number;
  claim: string;
  source_digest: string;
  claim_digest: string;
};
export type KnowledgeDeclarationRef = {
  snapshot: string;
  module_path: string;
  declaration_name_key: string;
  statement_id: string;
  type_digest: string;
};
export type KnowledgeQueryIntent = {
  id: string;
  subject: CoverageSubject;
  snapshot: string | null;
  index_revision: string | null;
  request_digest: string;
  query: string;
  scopes: string[];
  admitted_at: string;
};
export type KnowledgeQueryReceipt = Omit<KnowledgeQueryIntent, 'admitted_at'> & {
  protocol: string;
  state: 'complete' | 'partial' | 'timeout' | 'unavailable' | 'invalid_publication';
  response?: { id: string; bytes: number; sha256: string };
  returned: KnowledgeDeclarationRef[];
  limits: string[];
  retryable: boolean;
  started_at: string;
  completed_at: string;
};
export type ClaimCoverage = {
  subject: CoverageSubject;
  receipt_ids: string[];
  relation: CoverageRelation;
  qualification:
    | { level: 'candidate' }
    | { level: 'source_audited'; auditor: string; audit_id: string }
    | { level: 'certificate_claim'; protocol: string; artifact_digest: string };
  summary: string;
  limits: string[];
  binding: null | {
    premises: KnowledgeDeclarationRef[];
    relation: 'equivalent' | 'instance' | 'implies' | 'refutes' | 'proof_use';
    parameters: { parameter: string; expression: string }[];
    hypotheses: { hypothesis: string; argument: string }[];
    argument: string;
    unresolved: string[];
    source_lineage: string[];
  };
};
export type KnowledgeEvent =
  | { event: 'query_admitted'; intent: KnowledgeQueryIntent }
  | { event: 'query_completed'; receipt: KnowledgeQueryReceipt }
  | { event: 'coverage_assessed'; assessment: ClaimCoverage; at: string };

export interface ComputationReceipt {
  protocol: string;
  round: number;
  version: number;
  claims_revision: number;
  source_sha256: string;
  attachment: EvidenceAttachment;
  state: 'counterexample' | 'not_counterexample' | 'invalid_input' | 'arithmetic_limit';
  dimension: number | null;
  numerator: string | null;
  denominator: string | null;
  explanation: string;
  checked_at: string;
}

export type ResearchCandidate = {
  digest: string;
  subject: CoverageSubject;
  gap: { kind: 'hypothesis' | 'bridge'; index: number };
  unresolved_text: string;
  original_statement: string;
  receipt_ids: string[];
  binding: NonNullable<ClaimCoverage['binding']>;
};
export type ResearchGoal = {
  definitions: string[];
  hypotheses: string[];
  quantifiers: string;
  conclusion: string;
  correspondence: string;
};
export type NewResearchObligation = {
  round: number;
  claim: string;
  gap: ResearchCandidate['gap'];
  candidate_digest: string;
  goal: ResearchGoal;
  budget: { max_attempts: number; expires_at: string };
  consent: string;
};
export type ResearchAttempt = {
  id: string;
  target_digest: string;
  solution_digest: string;
  submitted_at: string;
  note: string;
  receipt: VerificationReceipt | null;
  invalidated: string | null;
};
export type ResearchAttemptInput = {
  target_digest: string;
  solution: string;
  modules?: Record<string, string>;
  note?: string;
};
export type ResearchView = {
  candidates: ResearchCandidate[];
  obligations: {
    state: string;
    target: LeanStatementAttempt | null;
    attempts: ResearchAttempt[];
    obligation: {
      id: string;
      candidate: ResearchCandidate;
      goal: ResearchGoal;
      budget: NewResearchObligation['budget'];
      authorized_at: string;
      authorized_by: string;
      consent: string;
    };
  }[];
};

export type KnowledgeCapabilities = {
  source_query_protocol: string;
  default_source_ranker: string;
  coverage_certificates: {
    supported: boolean;
    recognized_protocols: string[];
    reason_code: string;
    reason: string;
    required_bindings: string[];
    ordinary_lean_establishes_coverage: boolean;
  };
};

export type KnowledgeBinding = NonNullable<ClaimCoverage['binding']>;
export type BindingTarget = {
  subject: CoverageSubject;
  statement: string;
  sources: KnowledgeDeclarationRef[];
};
export type BindingOption = { digest: string; target: BindingTarget };
export type NewBindingBoard = {
  option_digest: string;
  sources: KnowledgeDeclarationRef[];
  context: string;
  license: string;
  consent: string;
};
export type NewBindingContribution = {
  binding: KnowledgeBinding;
  license: string;
  consent: string;
};
export type NewBindingChallenge = {
  contribution: string;
  evidence: string;
  license: string;
  consent: string;
};
export type BindingContribution = NewBindingContribution & {
  id: string;
  canonical: string;
  contributor: string;
  at: string;
};
export type BindingChallenge = NewBindingChallenge & { id: string; challenger: string; at: string };
export type PublicBindingAudit = {
  request: string;
  contribution: string;
  verdict: 'supported' | 'rejected' | 'unclear';
  evidence_complete: boolean;
  at: string;
};
export type BindingAudit = PublicBindingAudit & {
  auditor: string;
  reason: string;
  evidence: string;
  inspected_sources: KnowledgeDeclarationRef[];
};
export type BindingCredit = { canonical: string; contribution: string; contributor: string };
export type PublicBindingBoard = {
  id: string;
  target: BindingTarget;
  context: string;
  license: string;
  authorized_by: string;
  authorized_at: string;
  contributions: BindingContribution[];
  challenges: BindingChallenge[];
  audits: PublicBindingAudit[];
  credits: BindingCredit[];
};
export type BindingEvent =
  | { event: 'contributed'; contribution: BindingContribution }
  | { event: 'challenged'; challenge: BindingChallenge }
  | { event: 'audited'; audit: BindingAudit }
  | { event: 'revoked'; by: string; reason: string; at: string };
export type PrivateBindingView = {
  options: BindingOption[];
  boards: (Omit<PublicBindingBoard, 'contributions' | 'challenges' | 'audits' | 'credits'> & {
    consent: string;
    receipt_ids: string[];
    events: BindingEvent[];
  })[];
};
export type BindingDirectoryEntry = {
  id: string;
  submission: string;
  claim: string;
  statement: string;
  authorized_by: string;
  authorized_at: string;
  contributions: number;
  credits: number;
};

export type BindingProfileEntry = {
  submission: string;
  board: string;
  statement: string;
  contribution: BindingContribution;
  credited: boolean;
  latest_verdict: PublicBindingAudit['verdict'] | null;
};

export type FeedbackKind =
  | { kind: 'open_problem'; claim: string }
  | { kind: 'verified_solution'; claim: string; attempt: string }
  | { kind: 'library_binding'; board: string; contribution: string }
  | { kind: 'witness_formalization'; claim: string; witness: number }
  | {
      kind: 'dtr_registration';
      claim: string;
      attempt: string;
      slots: {
        escape_from: string;
        readout_via: string;
        realization: string;
        escape_continues: string;
      };
    }
  | { kind: 'erratum'; report: string; upstream: KnowledgeDeclarationRef[] };
export type FeedbackSource = {
  submission: string;
  record: string;
  version: number;
  claims_revision: number;
  source_digest: string;
};
export type FeedbackPreview = {
  digest: string;
  producer: string;
  source: FeedbackSource;
  kind: FeedbackKind;
  required_signers: string[];
  license: string;
  payload: Record<string, unknown>;
};
export type FeedbackObservation = {
  request: string;
  pull_request: number;
  upstream_repository: string;
  upstream_base: string;
  reference_state: string;
  merge_commit: string | null;
  merged_at: string | null;
  consumed_source_commit: string;
  consumed_source_tree: string;
  snapshot: string;
  index_revision: string;
  merge_in_consumed_source: boolean;
  declarations: {
    identity: KnowledgeDeclarationRef;
    present: boolean;
    source_sha256: string | null;
    freeze_status: string | null;
    frozen_node_id: string | null;
    package_marker_present: boolean;
  }[];
  evidence: Blob;
  observed_at: string;
  limits: string[];
};
export type FeedbackReadbackRequest = {
  pull_request: number;
  index_revision: string;
  declarations: KnowledgeDeclarationRef[];
  observation_key: string;
};
export type FeedbackEvent =
  | {
      event: 'authorized';
      by: string;
      digest: string;
      license: string;
      consent: string;
      rights_asserted: boolean;
      at: string;
    }
  | { event: 'revoked'; by: string; reason: string; at: string }
  | { event: 'exported'; by: string; at: string }
  | {
      event: 'readback_requested';
      id: string;
      request: FeedbackReadbackRequest;
      by: string;
      at: string;
    }
  | { event: 'observed'; observation: FeedbackObservation };
export type FeedbackProposal = {
  id: string;
  digest: string;
  producer: string;
  source: FeedbackSource;
  kind: FeedbackKind;
  payload: Blob;
  required_signers: string[];
  prepared_by: string;
  prepared_at: string;
  preparation_consent: string;
  events: FeedbackEvent[];
};
export type FeedbackView = {
  proposal: FeedbackProposal;
  payload: Record<string, unknown> | null;
  state: 'revoked' | 'source_or_evidence_changed' | 'authorized' | 'awaiting_authorizations';
};
export type FeedbackExport = {
  protocol: string;
  proposal: string;
  digest: string;
  payload: Record<string, unknown>;
  authorizations: FeedbackEvent[];
  proposal_markdown: string;
};

export type ReviewSummary = {
  round: number | null;
  version: number | null;
  claims_revision: number;
  delivered_at: string | null;
  in_progress: boolean;
  interrupted: boolean;
  has_execution_limits: boolean;
};
