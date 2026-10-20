import { useCallback, useEffect, useRef } from 'react';
import type { RefereeRound, Submission } from './types';
import { useApi } from './context';
import { useAsync } from './useAsync';
import { hasWaitingResponse, reviewDelivered } from '../lib/reviewProgress';

function isActive(round: RefereeRound): boolean {
  if (round.progress)
    return Object.values(round.progress).some(
      (step) => step.state === 'pending' || step.state === 'running',
    );
  return [round.referee, round.audit, round.advice, round.letter, round.formal].some(
    (step) =>
      step !== undefined && (step.state.state === 'pending' || step.state.state === 'running'),
  );
}

/** One referee resource shared by the paper standing and the feedback workspace. */
export function useReferee(submission: Submission, isStaff: boolean, enabled = true) {
  const api = useApi();
  const revision = submission.revision;
  const load = useCallback(
    (signal: AbortSignal) => {
      void revision;
      // Changing between the author and staff projections must reload the file.
      void isStaff;
      return api.referee(submission.id, { signal });
    },
    [api, submission.id, revision, isStaff],
  );
  const resource = useAsync(enabled ? load : null);
  const { reload } = resource;
  const latest = resource.value?.rounds
    .filter(
      (round) =>
        isStaff ||
        (round.version === submission.versions.at(-1)?.number &&
          round.claims_revision === submission.claims_revision),
    )
    .reduce<RefereeRound | undefined>(
      (latest, round) => (!latest || round.number > latest.number ? round : latest),
      undefined,
    );
  const active = isStaff && latest !== undefined && isActive(latest);
  const status = resource.state.status;
  const wasActive = useRef(false);
  useEffect(() => {
    // A temporary fetch failure must not stop a review that is still in progress.
    if (status === 'ok') wasActive.current = active;
    if (!enabled || !isStaff || !wasActive.current) return;
    const timer = window.setInterval(reload, 30_000);
    return () => window.clearInterval(timer);
  }, [active, enabled, isStaff, status, reload]);

  const paperStatus = submission.status.state;
  // Older projections lack hidden-stage states and retain the existing polling fallback.
  // Keep focus refresh after completion so newly admitted rounds and letters are discovered.
  const authorPolling =
    paperStatus === 'in_review' ||
    hasWaitingResponse(resource.value, submission) ||
    !latest?.progress ||
    isActive(latest) ||
    (!reviewDelivered(latest, resource.value?.letters ?? []) &&
      latest.progress.letter.state === 'done');
  useEffect(() => {
    // The decision precedes advice and delivery, so a declined paper still awaits its letter.
    if (
      !enabled ||
      isStaff ||
      (paperStatus !== 'in_review' &&
        paperStatus !== 'accepted' &&
        paperStatus !== 'not_accepted' &&
        paperStatus !== 'review_complete')
    )
      return;
    const timer = authorPolling ? window.setInterval(reload, 30_000) : undefined;
    window.addEventListener('focus', reload);
    return () => {
      window.clearInterval(timer);
      window.removeEventListener('focus', reload);
    };
  }, [enabled, isStaff, paperStatus, authorPolling, reload]);

  return { ...resource, active };
}

export type RefereeResource = ReturnType<typeof useReferee>;
