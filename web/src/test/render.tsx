import { render } from '@testing-library/react';
import type { ReactElement } from 'react';
import { vi } from 'vitest';
import { ApiError, type ApiClient } from '../api/client';
import { ApiProvider } from '../api/context';
import { PolicyProvider } from '../api/policy';
import type { Person } from '../api/types';
import { SessionProvider } from '../auth/session';
import { RouterProvider } from '../routing/router';
import { policy } from './fixtures';

const notFound = () =>
  Promise.reject(new ApiError({ status: 404, code: 'not_found', detail: 'not stubbed' }));
const empty = async () => ({ items: [], next_before: null });

/** A client whose every call fails unless overridden. */
export function fakeApi(overrides: Partial<ApiClient> = {}, person?: Person): ApiClient {
  const base: ApiClient = {
    session: vi.fn(async () =>
      person ? { authenticated: true as const, person } : { authenticated: false as const },
    ),
    logout: vi.fn(async () => undefined),
    policy: vi.fn(async () => policy),
    me: vi.fn(notFound),
    getCma: vi.fn(async () => ({
      enabled: false,
      setup_url: '',
      binding: {
        person: person?.id ?? '',
        subject: person?.id ?? '',
        revision: 0,
        generation: 0,
        status: 'disconnected' as const,
        reason: 'not_connected',
        workspace_id: null,
        sandbox_id: null,
        sandbox_profile: null,
        checked_at: null,
        created_at: '2026-10-11T00:00:00Z',
        updated_at: '2026-10-11T00:00:00Z',
      },
    })),
    setCmaWorkspace: vi.fn(notFound),
    disconnectCma: vi.fn(notFound),
    listPeople: vi.fn(empty),
    setRoles: vi.fn(notFound),
    reviewSummary: vi.fn(notFound),
    prepareSubmission: vi.fn(notFound),
    submitPrepared: vi.fn(notFound),
    createSubmission: vi.fn(notFound),
    listSubmissions: vi.fn(empty),
    getSubmission: vi.fn(notFound),
    confirmClaims: vi.fn(notFound),
    uploadVersion: vi.fn(notFound),
    withdrawSubmission: vi.fn(notFound),
    setContributors: vi.fn(notFound),
    setVisibility: vi.fn(notFound),
    getAnalysis: vi.fn(notFound),
    previewDecision: vi.fn(notFound),
    knowledgeCapabilities: vi.fn(notFound),
    bindings: vi.fn(empty),
    bindingProfile: vi.fn(empty),
    binding: vi.fn(notFound),
    feedbackObservation: vi.fn(notFound),
    feedbacks: vi.fn(async () => []),
    feedback: vi.fn(notFound),
    previewFeedback: vi.fn(notFound),
    prepareFeedback: vi.fn(notFound),
    authorizeFeedback: vi.fn(notFound),
    revokeFeedback: vi.fn(notFound),
    exportFeedback: vi.fn(notFound),
    feedbackReadback: vi.fn(notFound),
    privateBindings: vi.fn(async () => ({ options: [], boards: [] })),
    publishBinding: vi.fn(notFound),
    revokeBinding: vi.fn(notFound),
    contributeBinding: vi.fn(notFound),
    challengeBinding: vi.fn(notFound),
    research: vi.fn(async () => ({ candidates: [], obligations: [] })),
    adoptResearch: vi.fn(),
    stopResearch: vi.fn(),
    respondResearchTarget: vi.fn(),
    submitResearchAttempt: vi.fn(),
    researchAttempt: vi.fn(),
    recheckResearch: vi.fn(),
    referee: vi.fn(async (id: string) => ({ id, rounds: [], letters: [], revision: 0 })),
    supporters: vi.fn(async () => []),
    getStatement: vi.fn(notFound),
    reportError: vi.fn(notFound),
    errorReports: vi.fn(async () => []),
    errorNotifications: vi.fn(async () => []),
    submitStatementAttempt: vi.fn(notFound),
    myStatementAttempts: vi.fn(async () => []),
    respondLeanStatement: vi.fn(notFound),
    listConjectures: vi.fn(empty),
    getConjecture: vi.fn(notFound),
    submitAttempt: vi.fn(notFound),
    getAttempt: vi.fn(notFound),
    myAttempts: vi.fn(async () => []),
    leaderboard: vi.fn(async () => []),
    entrantProfile: vi.fn(notFound),
    agents: vi.fn(async () => []),
    createAgent: vi.fn(notFound),
    renameAgent: vi.fn(notFound),
    retireAgent: vi.fn(notFound),
    notifications: vi.fn(async () => []),
    respondFormalization: vi.fn(notFound),
    fileReport: vi.fn(notFound),
    judgeClaim: vi.fn(notFound),
    adoptJudgements: vi.fn(notFound),
    applyDecision: vi.fn(notFound),
    proposeFormalization: vi.fn(notFound),
    updateConjecture: vi.fn(notFound),
    generateTasks: vi.fn(notFound),
    respondToFinding: vi.fn(notFound),
    restartReferee: vi.fn(notFound),
    sendFeedback: vi.fn(notFound),
    formalizations: vi.fn(empty),
    formalizationOpportunities: vi.fn(empty),
    preflightFormalization: vi.fn(async () => ({
      modules: 0,
      source_bytes: 0,
      errors: [],
      warnings: [],
      dependencies: [],
      requires_adapter: false,
    })),
    submitFormalization: vi.fn(notFound),
    verifyFormalization: vi.fn(notFound),
    listPapers: vi.fn(empty),
    getPaper: vi.fn(notFound),
    listTasks: vi.fn(empty),
    getTask: vi.fn(notFound),
    leaseTask: vi.fn(notFound),
    releaseTask: vi.fn(notFound),
    submitContribution: vi.fn(notFound),
    listContributions: vi.fn(empty),
    reviewContribution: vi.fn(notFound),
    listContributors: vi.fn(async () => []),
    getDonation: vi.fn(async () => null),
    updateDonation: vi.fn(notFound),
  };
  return { ...base, ...overrides };
}

export function renderWithApp(ui: ReactElement, api: ApiClient) {
  return render(
    <ApiProvider client={api}>
      <SessionProvider>
        <PolicyProvider>
          <RouterProvider>{ui}</RouterProvider>
        </PolicyProvider>
      </SessionProvider>
    </ApiProvider>,
  );
}
