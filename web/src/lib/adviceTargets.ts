import type { ImprovementTarget, RefereeRound, Submission, TargetPriorWork } from '../api/types';
import { t } from '../i18n/locale';
import { manuscriptReferences } from './statements';

const PRIOR_WORK_LABELS: Record<TargetPriorWork['verdict'], string> = {
  known: 'Known target',
  specialization_of: 'Special case of an opened result',
  related: 'Related result',
  not_located: 'No matching statement located in the reported search',
};

export function rankedImprovements<T extends ImprovementTarget>(items: readonly T[]): T[] {
  return [...items].sort((a, b) => (a.rank ?? Infinity) - (b.rank ?? Infinity));
}

/** Exact target text is never shortened into a different mathematical statement. */
export function targetDetailsText(item: ImprovementTarget): string {
  const prior = item.prior_work;
  return [
    item.rank ? `**${t('Priority:')}** ${item.rank}` : '',
    item.target_statement ? `**${t('Target statement:')}** ${item.target_statement}` : '',
    item.feasibility ? `**${t('Feasibility:')}** ${item.feasibility}` : '',
    item.risk ? `**${t('Risk:')}** ${item.risk}` : '',
    prior
      ? [
          `**${t('Prior work on this target:')}** ${t(PRIOR_WORK_LABELS[prior.verdict])}`,
          prior.source,
          prior.location,
          prior.note,
          t('Sources are reported by the advisor, not independently verified by Mathesis.'),
        ]
          .filter(Boolean)
          .join('\n\n')
      : '',
  ]
    .filter(Boolean)
    .join('\n\n');
}

/** Included in both the detailed author report and its Markdown download. */
export function researchTargetsText(submission: Submission, round: RefereeRound): string {
  const targets = rankedImprovements(round.research_suggestions ?? [])
    .filter(
      (item) =>
        item.target_statement || item.feasibility || item.risk || item.prior_work || item.rank,
    )
    .map((item) => `### ${item.suggestion}\n\n${targetDetailsText(item)}`);
  return targets.length
    ? manuscriptReferences(
        `## ${t('Research targets')}\n\n${targets.join('\n\n')}`,
        submission.claims,
      )
    : '';
}
