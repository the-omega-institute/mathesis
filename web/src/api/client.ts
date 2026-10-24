import type {
  FeedbackKind,
  FeedbackPreview,
  FeedbackProposal,
  FeedbackView,
  FeedbackExport,
  FeedbackReadbackRequest,
} from './types';
import type { BindingProfileEntry } from './types';
import type {
  PrivateBindingView,
  PublicBindingBoard,
  NewBindingBoard,
  NewBindingContribution,
  NewBindingChallenge,
  BindingDirectoryEntry,
} from './types';
import type {
  KnowledgeCapabilities,
  ResearchView,
  NewResearchObligation,
  ResearchAttemptInput,
  ResearchAttempt,
  LeanPreflight,
} from './types';
import type {
  LeanDeposit,
  FormalizationOpportunity,
  NewLeanDeposit,
  MyCmaStatus,
  ErrorReport,
  NewErrorReport,
  StatementDetail,
  Supporter,
  ConjectureSummary,
  ConjectureDetail,
  Entrant,
  AttemptView,
  LeaderboardRow,
  EntrantProfile,
  SolveNotification,
  ClaimConfirmation,
  Contribution,
  ContributionMode,
  ContributionReview,
  ConjectureState,
  Credit,
  Decision,
  DonationGrant,
  DonationPatch,
  FileReport,
  Listing,
  NewContribution,
  NewJudgement,
  NewPaper,
  PaperAnalysis,
  PaperSummary,
  Person,
  PolicyDocument,
  Problem,
  ProblemCode,
  PublicPaper,
  ReportedStage,
  Role,
  Session,
  Submission,
  SubmittedContribution,
  Task,
  TaskContext,
  TaskGeneration,
  TaskKind,
  TaskState,
  ClaimJudgement,
  AnalysisVisibility,
  FeedbackLetter,
  RefereeFile,
  SendFeedback,
} from './types';

export const API_BASE = '/api/v1';

/** Codes the client adds for failures that never reached a problem document. */
export type ClientErrorCode = 'network' | 'unknown';
export type ApiErrorCode = ProblemCode | ClientErrorCode;

const KNOWN_CODES: readonly ProblemCode[] = [
  'not_authenticated',
  'forbidden',
  'not_found',
  'invalid',
  'conflict',
  'stale_revision',
  'unavailable',
];

const CODE_BY_STATUS: Readonly<{ [status: number]: ProblemCode }> = {
  401: 'not_authenticated',
  403: 'forbidden',
  404: 'not_found',
  409: 'conflict',
  422: 'invalid',
  503: 'unavailable',
};

/** A failed API call, carrying the server's problem document when there was one. */
export class ApiError extends Error {
  readonly status: number;
  readonly code: ApiErrorCode;
  readonly detail: string;
  readonly title: string | undefined;
  readonly type: string | undefined;

  constructor(init: {
    status: number;
    code: ApiErrorCode;
    detail: string;
    title?: string;
    type?: string;
  }) {
    super(init.detail);
    this.name = 'ApiError';
    this.status = init.status;
    this.code = init.code;
    this.detail = init.detail;
    this.title = init.title;
    this.type = init.type;
  }
}

export function isApiError(error: unknown): error is ApiError {
  return error instanceof ApiError;
}

function isProblemCode(value: unknown): value is ProblemCode {
  return typeof value === 'string' && (KNOWN_CODES as readonly string[]).includes(value);
}

function codeForStatus(status: number): ApiErrorCode {
  return CODE_BY_STATUS[status] ?? (status >= 500 ? 'unavailable' : 'unknown');
}

function asProblem(value: unknown): Problem | null {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) return null;
  const v = value as { [key: string]: unknown };
  const str = (key: string) => (typeof v[key] === 'string' ? (v[key] as string) : undefined);
  return {
    type: str('type'),
    title: str('title'),
    detail: str('detail'),
    code: str('code'),
    status: typeof v.status === 'number' ? v.status : undefined,
  };
}

/**
 * Turn a non-2xx response into an `ApiError`. A problem+json body (or any
 * JSON object with the same fields) supplies code and detail; anything else —
 * an HTML page from a proxy, an empty body — falls back to the HTTP status.
 */
export async function parseErrorResponse(response: Response): Promise<ApiError> {
  const contentType = response.headers.get('content-type') ?? '';
  let problem: Problem | null = null;
  if (contentType.includes('json')) {
    try {
      problem = asProblem(await response.json());
    } catch {
      problem = null;
    }
  }
  const rawCode = problem?.code;
  const code = isProblemCode(rawCode) ? rawCode : codeForStatus(response.status);
  const detail =
    problem?.detail?.trim() ||
    problem?.title?.trim() ||
    response.statusText ||
    `Request failed with status ${response.status}`;
  return new ApiError({
    status: response.status,
    code,
    detail,
    title: problem?.title,
    type: problem?.type,
  });
}

type QueryValue = string | number | undefined | null;

export function buildQuery(params: { [key: string]: QueryValue }): string {
  const search = new URLSearchParams();
  for (const [key, value] of Object.entries(params)) {
    if (value === undefined || value === null || value === '') continue;
    search.set(key, String(value));
  }
  const text = search.toString();
  return text === '' ? '' : `?${text}`;
}

export interface PaperQuery extends PageParams {
  q?: string;
  author?: string;
  msc?: string;
  kind?: 'paper' | 'note' | 'conjecture' | '';
}

export interface PageParams {
  before?: string | null;
  limit?: number;
}

export type SubmissionScope = 'mine' | 'queue';

export interface TaskQuery extends PageParams {
  kind?: TaskKind | '';
  status?: TaskState | '';
  submission?: string;
  holder?: string;
}

export interface ContributionQuery extends PageParams {
  task?: string;
  contributor?: string;
  status?: Contribution['status']['state'] | '';
  kind?: TaskKind | '';
}

export interface RequestOptions {
  signal?: AbortSignal;
}

export type FetchLike = (input: string, init?: RequestInit) => Promise<Response>;

/** The two multipart fields of `POST /submissions`. */
export const UPLOAD_FIELDS = { metadata: 'metadata', source: 'source', note: 'note' } as const;

/**
 * The multipart body of `POST /submissions`: `metadata` is the JSON text of
 * `NewPaper`, `source` the LaTeX file (.tex, .zip or .tar.gz).
 */
export function uploadBody(metadata: NewPaper, source?: File): FormData {
  const form = new FormData();
  form.append(UPLOAD_FIELDS.metadata, JSON.stringify(metadata));
  if (source) form.append(UPLOAD_FIELDS.source, source, source.name);
  return form;
}

/** The multipart body of `POST /submissions/{id}/versions`. */
export function versionBody(source: File, note: string, processingConsent?: string): FormData {
  const form = new FormData();
  form.append(UPLOAD_FIELDS.source, source, source.name);
  form.append(UPLOAD_FIELDS.note, note);
  if (processingConsent) form.append('processing_consent', processingConsent);
  return form;
}

export interface ApiClient {
  feedbackObservation(
    id: string,
    key: string,
    request: string,
    o?: RequestOptions,
  ): Promise<Record<string, unknown>>;
  feedbacks(id: string, o?: RequestOptions): Promise<FeedbackProposal[]>;
  feedback(id: string, key: string, o?: RequestOptions): Promise<FeedbackView>;
  previewFeedback(id: string, input: FeedbackKind): Promise<FeedbackPreview>;
  prepareFeedback(
    id: string,
    input: { kind: FeedbackKind; preview_digest: string; preparation_consent: string },
  ): Promise<FeedbackView>;
  authorizeFeedback(
    id: string,
    key: string,
    input: { digest: string; license: string; consent: string; rights_asserted: boolean },
  ): Promise<FeedbackView>;
  revokeFeedback(id: string, key: string, reason: string): Promise<{ revoked: boolean }>;
  exportFeedback(id: string, key: string): Promise<FeedbackExport>;
  feedbackReadback(id: string, key: string, input: FeedbackReadbackRequest): Promise<FeedbackView>;
  bindingProfile(
    person: string,
    before?: string,
    o?: RequestOptions,
  ): Promise<Listing<BindingProfileEntry>>;
  bindings(before?: string, o?: RequestOptions): Promise<Listing<BindingDirectoryEntry>>;
  binding(id: string, key: string, o?: RequestOptions): Promise<PublicBindingBoard>;
  privateBindings(id: string, o?: RequestOptions): Promise<PrivateBindingView>;
  publishBinding(id: string, input: NewBindingBoard): Promise<PublicBindingBoard>;
  revokeBinding(id: string, key: string, reason: string): Promise<{ revoked: boolean }>;
  contributeBinding(
    id: string,
    key: string,
    input: NewBindingContribution,
  ): Promise<PublicBindingBoard>;
  challengeBinding(
    id: string,
    key: string,
    input: NewBindingChallenge,
  ): Promise<PublicBindingBoard>;
  knowledgeCapabilities(o?: RequestOptions): Promise<KnowledgeCapabilities>;
  research(id: string, o?: RequestOptions): Promise<ResearchView>;
  adoptResearch(id: string, input: NewResearchObligation): Promise<ResearchView>;
  stopResearch(id: string, key: string, reason: string): Promise<ResearchView>;
  respondResearchTarget(
    id: string,
    key: string,
    input: { digest: string; confirm: boolean; comment?: string },
  ): Promise<ResearchView>;
  submitResearchAttempt(
    id: string,
    key: string,
    input: ResearchAttemptInput,
  ): Promise<ResearchView>;
  researchAttempt(
    id: string,
    key: string,
    attempt: string,
  ): Promise<{ attempt: ResearchAttempt; solution: string; modules: Record<string, string> }>;
  recheckResearch(id: string, key: string, attempt: string): Promise<{ requested: boolean }>;

  getCma(options?: RequestOptions): Promise<MyCmaStatus>;
  setCmaWorkspace(input: { workspace_id: string; expected_revision: number }): Promise<MyCmaStatus>;
  disconnectCma(): Promise<MyCmaStatus>;
  session(options?: RequestOptions): Promise<Session>;
  logout(): Promise<void>;
  policy(options?: RequestOptions): Promise<PolicyDocument>;
  me(options?: RequestOptions): Promise<Person>;
  listPeople(page?: PageParams, options?: RequestOptions): Promise<Listing<Person>>;
  setRoles(id: string, roles: Role[]): Promise<Person>;

  // Authors.
  createSubmission(metadata: NewPaper, source?: File): Promise<Submission>;
  listSubmissions(
    scope: SubmissionScope,
    page?: PageParams,
    options?: RequestOptions,
  ): Promise<Listing<Submission>>;
  getSubmission(id: string, options?: RequestOptions): Promise<Submission>;
  confirmClaims(id: string, claims: ClaimConfirmation[]): Promise<Submission>;
  uploadVersion(
    id: string,
    source: File,
    note: string,
    processingConsent?: string,
  ): Promise<Submission>;
  withdrawSubmission(id: string): Promise<Submission>;
  setContributors(id: string, open: boolean): Promise<Submission>;
  setVisibility(
    id: string,
    visibility: Exclude<AnalysisVisibility, 'undecided'>,
  ): Promise<Submission>;
  getAnalysis(id: string, options?: RequestOptions): Promise<PaperAnalysis>;
  previewDecision(id: string, options?: RequestOptions): Promise<Decision>;
  referee(id: string, options?: RequestOptions): Promise<RefereeFile>;
  respondFormalization(
    id: string,
    claim: string,
    response: { approve: boolean; reason?: string },
  ): Promise<Submission>;

  respondLeanStatement(
    id: string,
    response: { digest: string; confirm: boolean; comment?: string },
  ): Promise<Submission>;

  // Editors.
  fileReport(id: string, stage: ReportedStage, report: FileReport): Promise<Submission>;
  judgeClaim(id: string, claim: string, judgement: NewJudgement): Promise<ClaimJudgement>;
  adoptJudgements(id: string): Promise<Submission>;
  applyDecision(id: string): Promise<Submission>;
  proposeFormalization(id: string, claim: string, reason: string): Promise<Submission>;
  updateConjecture(id: string, claim: string, state: ConjectureState): Promise<Submission>;
  generateTasks(id: string): Promise<TaskGeneration>;
  respondToFinding(
    id: string,
    response: import('./types').NewFindingResponse,
  ): Promise<RefereeFile>;
  restartReferee(id: string): Promise<RefereeFile>;
  sendFeedback(id: string, letter: SendFeedback): Promise<FeedbackLetter>;

  supporters(options?: RequestOptions): Promise<Supporter[]>;
  getStatement(record: string, claim: string, options?: RequestOptions): Promise<StatementDetail>;
  reportError(record: string, body: NewErrorReport): Promise<ErrorReport>;
  errorReports(record: string, options?: RequestOptions): Promise<ErrorReport[]>;
  errorNotifications(options?: RequestOptions): Promise<ErrorReport[]>;
  submitStatementAttempt(
    record: string,
    claim: string,
    solution: string,
    as_agent?: string,
    note?: string,
  ): Promise<AttemptView>;
  myStatementAttempts(
    record: string,
    claim: string,
    options?: RequestOptions,
  ): Promise<AttemptView[]>;
  listConjectures(
    page?: PageParams & { status?: 'open' | 'solved' | 'disproved' },
    options?: RequestOptions,
  ): Promise<Listing<ConjectureSummary>>;

  getConjecture(record: string, claim: string, options?: RequestOptions): Promise<ConjectureDetail>;
  submitAttempt(
    record: string,
    claim: string,
    solution: string,
    as_agent?: string,
    note?: string,
  ): Promise<AttemptView>;
  getAttempt(id: string, options?: RequestOptions): Promise<AttemptView>;
  myAttempts(record: string, claim: string, options?: RequestOptions): Promise<AttemptView[]>;
  leaderboard(
    period?: 'all' | 'month',
    entrants?: 'all' | 'people' | 'agents',
    options?: RequestOptions,
  ): Promise<LeaderboardRow[]>;
  entrantProfile(id: string, options?: RequestOptions): Promise<EntrantProfile>;
  agents(options?: RequestOptions): Promise<Entrant[]>;
  createAgent(name: string): Promise<Entrant>;
  renameAgent(id: string, name: string): Promise<Entrant>;
  retireAgent(id: string): Promise<Entrant>;
  notifications(options?: RequestOptions): Promise<SolveNotification[]>;

  formalizationOpportunities(
    query?: PageParams & { q?: string; kind?: string; status?: string },
    options?: RequestOptions,
  ): Promise<Listing<FormalizationOpportunity>>;
  submitFormalization(body: NewLeanDeposit): Promise<LeanDeposit>;
  preflightFormalization(body: NewLeanDeposit): Promise<LeanPreflight>;
  formalizations(page?: PageParams, options?: RequestOptions): Promise<Listing<LeanDeposit>>;
  verifyFormalization(
    id: string,
    record: string,
    claim: string,
    expectedTarget?: string,
    solution?: string,
  ): Promise<AttemptView>;
  // Public papers.
  listPapers(page?: PaperQuery, options?: RequestOptions): Promise<Listing<PaperSummary>>;
  getPaper(record: string, options?: RequestOptions): Promise<PublicPaper>;

  // Contributors.
  listTasks(query?: TaskQuery, options?: RequestOptions): Promise<Listing<Task>>;
  getTask(id: string, options?: RequestOptions): Promise<TaskContext>;
  leaseTask(id: string, mode?: ContributionMode): Promise<Task>;
  releaseTask(id: string): Promise<Task>;
  submitContribution(id: string, contribution: NewContribution): Promise<SubmittedContribution>;
  listContributions(
    query?: ContributionQuery,
    options?: RequestOptions,
  ): Promise<Listing<Contribution>>;
  reviewContribution(id: string, review: ContributionReview): Promise<Contribution>;
  listContributors(contributor?: string, options?: RequestOptions): Promise<Credit[]>;
  getDonation(options?: RequestOptions): Promise<DonationGrant | null>;
  updateDonation(patch: DonationPatch): Promise<DonationGrant>;
}

const seg = encodeURIComponent;

export function createApiClient(fetchImpl: FetchLike = (input, init) => fetch(input, init)) {
  async function request<T>(
    method: 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE',
    path: string,
    body?: unknown,
    options?: RequestOptions,
  ): Promise<T> {
    const headers: { [key: string]: string } = { Accept: 'application/json' };
    // FormData sets its own multipart Content-Type with the boundary.
    const multipart = typeof FormData !== 'undefined' && body instanceof FormData;
    if (body !== undefined && !multipart) headers['Content-Type'] = 'application/json';
    let response: Response;
    try {
      response = await fetchImpl(path, {
        method,
        headers,
        body:
          body === undefined ? undefined : multipart ? (body as FormData) : JSON.stringify(body),
        credentials: 'same-origin',
        signal: options?.signal,
      });
    } catch (error) {
      if (error instanceof DOMException && error.name === 'AbortError') throw error;
      throw new ApiError({
        status: 0,
        code: 'network',
        detail:
          'The server could not be reached. The request could not be confirmed; reload the page data when your connection is back to see the current state.',
      });
    }
    if (!response.ok) throw await parseErrorResponse(response);
    if (response.status === 204) return undefined as T;
    const text = await response.text();
    if (text === '') return undefined as T;
    try {
      return JSON.parse(text) as T;
    } catch {
      throw new ApiError({
        status: response.status,
        code: 'unknown',
        detail: 'The server returned a response that is not JSON.',
      });
    }
  }

  const api = (path: string) => `${API_BASE}${path}`;
  const sub = (id: string, rest = '') => api(`/submissions/${seg(id)}${rest}`);
  const page = (p: PageParams) => ({ before: p.before, limit: p.limit });

  const client: ApiClient = {
    feedbackObservation: (id, key, r, o) =>
      request('GET', sub(id, `/feedback/${seg(key)}/observations/${seg(r)}`), undefined, o),
    feedbacks: (id, o) => request('GET', sub(id, '/feedback'), undefined, o),
    feedback: (id, key, o) => request('GET', sub(id, `/feedback/${seg(key)}`), undefined, o),
    previewFeedback: (id, input) => request('POST', sub(id, '/feedback/preview'), input),
    prepareFeedback: (id, input) => request('POST', sub(id, '/feedback'), input),
    authorizeFeedback: (id, key, input) =>
      request('POST', sub(id, `/feedback/${seg(key)}/authorize`), input),
    revokeFeedback: (id, key, reason) =>
      request('POST', sub(id, `/feedback/${seg(key)}/revoke`), { reason }),
    exportFeedback: (id, key) => request('POST', sub(id, `/feedback/${seg(key)}/export`), {}),
    feedbackReadback: (id, key, input) =>
      request('POST', sub(id, `/feedback/${seg(key)}/readback`), input),
    bindingProfile: (person, before, o) =>
      request(
        'GET',
        api(`/binding-contributors/${seg(person)}`) + (before ? '?before=' + seg(before) : ''),
        undefined,
        o,
      ),
    bindings: (before, o) =>
      request('GET', api('/bindings') + (before ? '?before=' + seg(before) : ''), undefined, o),
    binding: (id, key, o) => request('GET', api(`/bindings/${seg(id)}/${seg(key)}`), undefined, o),
    privateBindings: (id, o) => request('GET', sub(id, '/bindings'), undefined, o),
    publishBinding: (id, input) => request('POST', sub(id, '/bindings'), input),
    revokeBinding: (id, key, reason) =>
      request('POST', sub(id, `/bindings/${seg(key)}/revoke`), { reason }),
    contributeBinding: (id, key, input) =>
      request('POST', api(`/bindings/${seg(id)}/${seg(key)}/contributions`), input),
    challengeBinding: (id, key, input) =>
      request('POST', api(`/bindings/${seg(id)}/${seg(key)}/challenges`), input),
    knowledgeCapabilities: (o) => request('GET', api('/knowledge/capabilities'), undefined, o),
    research: (id, o) => request('GET', sub(id, '/research'), undefined, o),
    adoptResearch: (id, input) => request('POST', sub(id, '/research'), input),
    stopResearch: (id, key, reason) =>
      request('POST', sub(id, `/research/${seg(key)}/stop`), { reason }),
    respondResearchTarget: (id, key, input) =>
      request('POST', sub(id, `/research/${seg(key)}/target/response`), input),
    submitResearchAttempt: (id, key, input) =>
      request('POST', sub(id, `/research/${seg(key)}/attempts`), input),
    researchAttempt: (id, key, attempt) =>
      request('GET', sub(id, `/research/${seg(key)}/attempts/${seg(attempt)}`)),
    recheckResearch: (id, key, attempt) =>
      request('POST', sub(id, `/research/${seg(key)}/attempts/${seg(attempt)}/recheck`)),
    session: (o) => request('GET', '/auth/session', undefined, o),
    logout: () => request('POST', '/auth/logout'),
    policy: (o) => request('GET', api('/policy'), undefined, o),
    me: (o) => request('GET', api('/me'), undefined, o),
    listPeople: (p = {}, o) => request('GET', api(`/people${buildQuery(page(p))}`), undefined, o),
    setRoles: (id, roles) => request('PUT', api(`/people/${seg(id)}/roles`), { roles }),

    createSubmission: (metadata, source) =>
      request('POST', api('/submissions'), uploadBody(metadata, source)),
    listSubmissions: (scope, p = {}, o) =>
      request('GET', api(`/submissions${buildQuery({ scope, ...page(p) })}`), undefined, o),
    getSubmission: (id, o) => request('GET', sub(id), undefined, o),
    confirmClaims: (id, claims) => request('POST', sub(id, '/claims'), { claims }),
    uploadVersion: (id, source, note, processingConsent) =>
      request('POST', sub(id, '/versions'), versionBody(source, note, processingConsent)),
    withdrawSubmission: (id) => request('POST', sub(id, '/withdraw')),
    setContributors: (id, open) => request('PUT', sub(id, '/contributors'), { open }),
    setVisibility: (id, visibility) => request('PUT', sub(id, '/visibility'), { visibility }),
    getAnalysis: (id, o) => request('GET', sub(id, '/analysis'), undefined, o),
    previewDecision: (id, o) => request('GET', sub(id, '/decision'), undefined, o),
    referee: (id, o) => request('GET', sub(id, '/referee'), undefined, o),
    respondFormalization: (id, claim, response) =>
      request('POST', sub(id, `/formalization/items/${seg(claim)}/response`), response),

    respondLeanStatement: (id, response) =>
      request('POST', sub(id, '/lean-statement/response'), response),

    fileReport: (id, stage, report) =>
      request('POST', sub(id, `/stages/${seg(stage)}/reports`), report),
    judgeClaim: (id, claim, judgement) =>
      request('POST', sub(id, `/claims/${seg(claim)}/judgements`), judgement),
    adoptJudgements: (id) => request('POST', sub(id, '/escape/adopt')),
    applyDecision: (id) => request('POST', sub(id, '/decision')),
    proposeFormalization: (id, claim, reason) =>
      request('POST', sub(id, '/formalization/items'), { claim, reason }),
    updateConjecture: (id, claim, state) =>
      request('PUT', sub(id, `/conjectures/${seg(claim)}`), { state }),
    generateTasks: (id) => request('POST', sub(id, '/tasks')),
    respondToFinding: (id, response) => request('POST', sub(id, '/review/responses'), response),
    restartReferee: (id) => request('POST', sub(id, '/referee/restart')),
    sendFeedback: (id, letter) => request('POST', sub(id, '/referee/letters'), letter),

    getConjecture: (r, c, o) =>
      request('GET', api(`/conjectures/${seg(r)}/${seg(c)}`), undefined, o),
    submitAttempt: (r, c, solution, as_agent, note = '') =>
      request('POST', api(`/conjectures/${seg(r)}/${seg(c)}/attempts`), {
        solution,
        as_agent,
        note,
      }),
    getAttempt: (id, o) => request('GET', api(`/attempts/${seg(id)}`), undefined, o),
    myAttempts: (r, c, o) =>
      request('GET', api(`/conjectures/${seg(r)}/${seg(c)}/attempts/mine`), undefined, o),
    leaderboard: (period = 'all', entrants = 'all', o) =>
      request('GET', api(`/leaderboard${buildQuery({ period, entrants })}`), undefined, o),
    entrantProfile: (id, o) => request('GET', api(`/entrants/${seg(id)}`), undefined, o),
    agents: (o) => request('GET', api('/me/agents'), undefined, o),
    createAgent: (name) => request('POST', api('/me/agents'), { name }),
    renameAgent: (id, name) => request('PATCH', api(`/me/agents/${seg(id)}`), { name }),
    retireAgent: (id) => request('DELETE', api(`/me/agents/${seg(id)}`)),
    notifications: (o) => request('GET', api('/me/notifications'), undefined, o),
    supporters: (o) => request('GET', api('/supporters'), undefined, o),
    getStatement: (r, c, o) =>
      request('GET', api(`/papers/${seg(r)}/statements/${seg(c)}`), undefined, o),
    reportError: (r, b) => request('POST', api(`/papers/${seg(r)}/errors`), b),
    errorReports: (r, o) => request('GET', api(`/papers/${seg(r)}/errors`), undefined, o),
    errorNotifications: (o) => request('GET', api('/me/error-notifications'), undefined, o),
    submitStatementAttempt: (r, c, solution, as_agent, note) =>
      request('POST', api(`/papers/${seg(r)}/statements/${seg(c)}/attempts`), {
        solution,
        as_agent,
        note,
      }),
    myStatementAttempts: (r, c, o) =>
      request('GET', api(`/papers/${seg(r)}/statements/${seg(c)}/attempts/mine`), undefined, o),
    listConjectures: (p = {}, o) =>
      request(
        'GET',
        api(`/conjectures${buildQuery({ ...page(p), status: p.status })}`),
        undefined,
        o,
      ),
    formalizationOpportunities: (p = {}, o) =>
      request(
        'GET',
        api(
          `/formalizations/opportunities${buildQuery({ ...page(p), q: p.q, kind: p.kind, status: p.status })}`,
        ),
        undefined,
        o,
      ),
    preflightFormalization: (body) => request('POST', api('/formalizations/preflight'), body),
    submitFormalization: (body) => request('POST', api('/formalizations'), body),
    formalizations: (p = {}, o) =>
      request('GET', api(`/formalizations${buildQuery(page(p))}`), undefined, o),
    verifyFormalization: (id, record, claim, expectedTarget, solution) =>
      request('POST', api(`/formalizations/${encodeURIComponent(id)}/attempts`), {
        record,
        claim,
        expected_target: expectedTarget,
        solution,
      }),
    listPapers: (p = {}, o) =>
      request(
        'GET',
        api(
          `/papers${buildQuery({ ...page(p), q: p.q, author: p.author, msc: p.msc, kind: p.kind })}`,
        ),
        undefined,
        o,
      ),
    getPaper: (record, o) => request('GET', api(`/papers/${seg(record)}`), undefined, o),

    listTasks: (q = {}, o) =>
      request(
        'GET',
        api(
          `/tasks${buildQuery({
            kind: q.kind,
            status: q.status,
            submission: q.submission,
            holder: q.holder,
            ...page(q),
          })}`,
        ),
        undefined,
        o,
      ),
    getTask: (id, o) => request('GET', api(`/tasks/${seg(id)}`), undefined, o),
    leaseTask: (id, mode) =>
      request('POST', api(`/tasks/${seg(id)}/lease`), mode === undefined ? {} : { mode }),
    releaseTask: (id) => request('DELETE', api(`/tasks/${seg(id)}/lease`)),
    submitContribution: (id, c) => request('POST', api(`/tasks/${seg(id)}/contributions`), c),
    listContributions: (q = {}, o) =>
      request(
        'GET',
        api(
          `/contributions${buildQuery({
            task: q.task,
            contributor: q.contributor,
            status: q.status,
            kind: q.kind,
            ...page(q),
          })}`,
        ),
        undefined,
        o,
      ),
    reviewContribution: (id, review) =>
      request('POST', api(`/contributions/${seg(id)}/review`), review),
    listContributors: (contributor, o) =>
      request('GET', api(`/contributors${buildQuery({ contributor })}`), undefined, o),
    getCma: (o) => request('GET', api('/me/cma'), undefined, o),
    setCmaWorkspace: (input) => request('PUT', api('/me/cma'), input),
    disconnectCma: () => request('DELETE', api('/me/cma')),
    getDonation: (o) => request('GET', api('/donation'), undefined, o),
    updateDonation: (patch) => request('PATCH', api('/donation'), patch),
  };
  return client;
}

/** `GET /submissions/{id}/files/pdf?version=N`: inline PDF. */
export function pdfHref(submission: string, version?: number): string {
  return `${API_BASE}/submissions/${seg(submission)}/files/pdf${buildQuery({ version })}`;
}

/** `GET /submissions/{id}/files/source?version=N`: the uploaded archive. */
export function sourceFileHref(submission: string, version?: number): string {
  return `${API_BASE}/submissions/${seg(submission)}/files/source${buildQuery({ version })}`;
}

/** Leave the app for the NyxID sign-in flow, coming back to `path`. */
export function signInHref(path: string): string {
  return `/auth/login?return_to=${encodeURIComponent(path)}`;
}

/** Local development only: sign in as `dev:<name>` without a provider. */
export function devSignInHref(name: string, path: string): string {
  return `/auth/login${buildQuery({ as: name, return_to: path })}`;
}

/**
 * Leave the app to grant donated model quota through NyxID incremental
 * consent (`GET /auth/donate`). A full page navigation, never a fetch.
 */
export function donateHref(cap: number, model: string, returnTo: string): string {
  return `/auth/donate${buildQuery({ cap, model: model.trim(), return_to: returnTo })}`;
}

export function signIn(path: string = currentPath()): void {
  window.location.href = signInHref(path);
}

export function currentPath(): string {
  return `${window.location.pathname}${window.location.search}`;
}

/** A human-readable message for any thrown value. */
export function errorMessage(error: unknown): string {
  if (error instanceof ApiError) return error.detail;
  if (error instanceof Error) return error.message;
  return 'Something went wrong.';
}
