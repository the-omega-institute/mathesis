import { t } from '../i18n/locale';
import type { RefereeRound } from '../api/types';
import { AUTHOR_STEP_STATE_LABELS } from '../lib/labels';
import { REVIEW_STAGES } from '../lib/reviewProgress';
import { DateText } from './ui';

export function ReviewProgress({
  round,
  includeFormal,
}: {
  round: RefereeRound;
  includeFormal: boolean;
}) {
  if (!round.progress) return null;
  return (
    <div>
      <p className="small muted">
        {t('Round ')}
        {round.number} {t(' · source version ')}
        {round.version} {t(' · started')} <DateText iso={round.started_at} withTime />
      </p>
      <ol className="referee-progress" aria-label={t('Review stage details')}>
        {REVIEW_STAGES.map(([key, label]) => {
          const step = round.progress![key];
          if (key === 'formal' && (!includeFormal || step.state === 'skipped')) return null;
          return (
            <li
              key={key}
              className={`step-${step.state}`}
              aria-current={step.state === 'running' ? 'step' : undefined}
            >
              <strong>{t(label)}</strong>
              <span>{t(AUTHOR_STEP_STATE_LABELS[step.state])}</span>
              {step.state === 'running' ? (
                <span>
                  {t('Since ')}
                  <DateText iso={step.since} withTime />
                </span>
              ) : step.state === 'done' || step.state === 'failed' ? (
                <DateText iso={step.at} withTime />
              ) : null}
            </li>
          );
        })}
      </ol>
    </div>
  );
}
