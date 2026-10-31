import type { FeedbackLetter, RefereeRound, RejectReason, Submission } from '../api/types';
import { formalizationAdvice, formalizationAdviceText } from './formalizationAdvice';
import { manuscriptReferences, resultLabel } from './statements';

export function publicationReason(reason: RejectReason, submission?: Submission): string {
  const claim =
    'claim' in reason ? submission?.claims.find((item) => item.id === reason.claim) : undefined;
  const name = claim ? resultLabel(claim) : 'A main result';
  let text: string;
  switch (reason.reason) {
    case 'known_result':
      text = `${name} already follows from ${reason.prior.locator}.`;
      break;
    case 'bind_only':
      text =
        'The main results follow from existing results without establishing a distinct new mathematical contribution.';
      break;
    case 'no_main_result':
      text = 'The manuscript does not establish a proved main result.';
      break;
    case 'incorrect_result':
      text = `${name} has a proof gap or mathematical error. The analysis below explains the finding.`;
      break;
    case 'unchecked_result':
      text = `The correctness of ${name} has not been established by this review. This is a limitation of the review, not a demonstrated mathematical error.`;
      break;
    case 'insufficient_evidence': {
      // These are the venue's stored policy explanations, not model-written prose.
      // Preserve any mathematical qualification after the standard policy sentence.
      if (reason.detail.includes('has an undetermined proof-escape reading;')) {
        text = `The review could not establish a distinct new mathematical contribution in ${name}. Further mathematical evidence is needed.`;
      } else if (
        reason.detail.includes('is covered by the source-audited pinned knowledge comparison')
      ) {
        text = `${name} follows from the compared prior results. A claimed resolution of an open problem needs separate mathematical justification.`;
      } else {
        text = reason.detail.replace(
          /lacks sufficient evidence for its prerequisite comparison, new content or claimed open-problem settlement\./,
          'does not yet have sufficient support for its relationship to prior results, its claimed new contribution or its resolution of an open problem.',
        );
      }
      break;
    }
    case 'insufficient_utility':
      text = reason.detail.replace(
        /has no established independent utility under its mathematical or computational category:/,
        'does not yet have a demonstrated mathematical use or significance:',
      );
      break;
    case 'conjecture':
      text = reason.detail.replace(
        'would only re-bind known results:',
        'follows from known results:',
      );
      break;
    default:
      text = reason.detail;
  }
  return manuscriptReferences(text, submission?.claims ?? []);
}

/** Outcome comes only from the manuscript's applied state, never a model recommendation. */
export function reportOpening(s: Submission): string | undefined {
  if (s.status.state === 'review_complete')
    return 'Your private review is complete. The analysis and suggestions below are for your research; no publication decision has been made.';
  if (s.status.state === 'accepted') {
    if (s.analysis_visibility !== 'public')
      return 'Congratulations! Your manuscript has been accepted by Mathesis. It remains private under your current visibility setting.';
    return s.kind === 'conjecture'
      ? 'Congratulations! Your conjecture has been accepted for display on Mathesis.'
      : 'Congratulations! Your paper has been accepted and is now displayed on Mathesis.';
  }
  if (s.status.state === 'not_accepted')
    return 'We are sorry that we cannot accept your manuscript for display on Mathesis at present. The reasons and mathematical analysis are given below.';
  return undefined;
}

const headings = ['Review analysis', 'Suggestions', 'Further directions'] as const;

/** Contract 3 gives mathematical sections only. Legacy letters retain their private original. */
function authoredSections(round: RefereeRound, letter: FeedbackLetter) {
  if ((round.letter_contract ?? 0) < 3) return undefined;
  const matches = [
    ...letter.body.matchAll(/^## (Review analysis|Suggestions|Further directions)\s*$/gm),
  ];
  if (matches.length !== headings.length || matches.some((match, i) => match[1] !== headings[i]))
    return undefined;
  const sections = matches.map((match, i) =>
    letter.body.slice(match.index! + match[0].length, matches[i + 1]?.index).trim(),
  );
  // A malformed new report must not reintroduce an internal recommendation or outcome.
  const outcomeProse =
    /\b(?:minor|major)[ -]revision\b|\brevise.and.resubmit\b|\b(?:decision|publication)\s+(?:is\s+)?pending\b|\byour (?:paper|manuscript) (?:is|has been) (?:not )?(?:accepted|rejected)\b/i;
  return sections.every(Boolean) && !outcomeProse.test([...sections, letter.note].join('\n'))
    ? sections
    : undefined;
}

export function authorReport(
  s: Submission,
  round: RefereeRound,
  letter: FeedbackLetter,
): string | undefined {
  const opening = reportOpening(s);
  if (!opening) return undefined;
  const audit = round.audit?.state.state === 'done' ? round.audit.state.result : undefined;
  const reasons =
    s.status.state === 'not_accepted' && s.decision?.decision === 'not_accepted'
      ? s.decision.reasons.map((reason) => `- ${publicationReason(reason, s)}`).join('\n')
      : '';
  const authored = authoredSections(round, letter);
  const analysis = audit?.claims
    .flatMap((reading) => {
      const claim = s.claims.find((item) => item.id === reading.claim);
      if (
        !claim ||
        !reading.comment.trim() ||
        (claim.role !== 'main' && !['gap', 'error'].includes(reading.correctness))
      )
        return [];
      return [
        `### ${resultLabel(claim)}\n\n${reading.comment}${reading.known ? `\n\nRelated work: ${reading.known}` : ''}`,
      ];
    })
    .join('\n\n');
  const concerns = audit?.concerns
    .filter((item) => item.status !== 'refuted')
    .map(
      (item) =>
        `${item.status === 'not_checkable' ? '**Please check:** ' : ''}${item.concern}\n\n${item.note}`,
    )
    .join('\n\n');
  const suggestions = (research: boolean) =>
    round.research_suggestions
      ?.filter((item) => ['strengthen', 'generalize'].includes(item.kind) === research)
      .map(
        (item) =>
          `- ${item.suggestion}${item.excerpt ? '\n\n  This is an excerpt of the suggestion.' : ''}`,
      )
      .join('\n\n');
  const sections = authored ?? [
    analysis || 'No detailed mathematical analysis is available in this report.',
    [concerns, suggestions(false)].filter(Boolean).join('\n\n') ||
      'No specific improvements were identified in the available review.',
    suggestions(true) ||
      'No specific extension was established as a recommendation in this review.',
  ];
  const mathematical = headings.map((heading, i) => `## ${heading}\n\n${sections[i]}`).join('\n\n');
  return [
    opening,
    reasons ? `## Reasons for the decision\n\n${reasons}` : '',
    manuscriptReferences(mathematical, s.claims),
    authored && letter.note.trim()
      ? `## Supporting analysis\n\n${manuscriptReferences(letter.note, s.claims)}`
      : '',
    formalizationAdviceText(s, round),
  ]
    .filter(Boolean)
    .join('\n\n');
}

const internalOrOutcome =
  /\b(?:minor|major)[ -]revision\b|\brevise.and.resubmit\b|\b(?:decision|publication|review)\s+(?:is\s+)?(?:pending|complete)\b|\b(?:accepted|rejected|audit|audited|pipeline|coverage|receipt|query|queries|Codex|GPT|Orchestra|source-audited|bind.only|proof.escape)\b/i;

/** Use a whole existing mathematical paragraph; never cut a proof or generate a new claim. */
function overview(s: Submission, round: RefereeRound, letter: FeedbackLetter): string {
  const audit = round.audit?.state.state === 'done' ? round.audit.state.result : undefined;
  const analysis = authoredSections(round, letter)?.[0];
  const paragraphs = (analysis ?? letter.body).split(/\n\s*\n/).map((text) => text.trim());
  const paragraph = paragraphs.find(
    (text) =>
      text.split(/\s+/).length >= 20 &&
      text.split(/\s+/).length <= 110 &&
      !/^(?:Dear |Sincerely|#|[-*]|\d+[.)])/.test(text) &&
      !/\b(?:please|suggest|recommend|should|must)\b/i.test(text) &&
      !internalOrOutcome.test(text),
  );
  const summary = audit?.summary.trim();
  let text =
    paragraph ??
    (summary && summary.split(/\s+/).length <= 110 && !internalOrOutcome.test(summary)
      ? summary
      : undefined);
  if (!text) {
    const main = s.claims.filter((claim) => claim.role === 'main');
    const readings = main.map((claim) => audit?.claims.find((item) => item.claim === claim.id));
    text =
      readings.length && readings.every((item) => item?.correctness === 'correct')
        ? 'The review supports the main mathematical results. The recommendations below address the manuscript and possible next steps.'
        : 'The review findings and recommendations are available below. The detailed report explains the arguments and remaining uncertainties.';
  }
  // A positive overview never conceals a saved proof concern or an unchecked main result.
  if (audit?.claims.some((item) => ['gap', 'error'].includes(item.correctness)))
    text +=
      '\n\nThe review also identifies a proof gap or error; see the detailed findings before relying on the affected result.';
  if (
    audit &&
    s.claims.some(
      (claim) =>
        claim.role === 'main' &&
        !audit.claims.some((item) => item.claim === claim.id && item.correctness !== 'not_checked'),
    )
  )
    text +=
      '\n\nSome main results could not be fully checked. The detailed report identifies the limitations.';
  return manuscriptReferences(text, s.claims);
}

/** A short reading view, with all original findings retained in the expandable full report. */
export function authorReportPreview(
  s: Submission,
  round: RefereeRound,
  letter: FeedbackLetter,
): string | undefined {
  const opening = reportOpening(s);
  if (!opening) return undefined;
  const audit = round.audit?.state.state === 'done' ? round.audit.state.result : undefined;
  const priorities: Record<string, number> = {
    gap: 0,
    literature: 1,
    exposition: 2,
    computation: 3,
  };
  const improvements = (round.research_suggestions ?? [])
    .filter((item) => !['strengthen', 'generalize'].includes(item.kind))
    .sort((a, b) => priorities[a.kind]! - priorities[b.kind]!);
  const concise = (text: string) =>
    text.trim().split(/\s+/).length <= 85 &&
    !internalOrOutcome.test(text) &&
    !/[a-f0-9]{40,64}/i.test(text);
  const concerns =
    audit?.concerns
      .filter((item) => item.status !== 'refuted')
      .map(
        (item) =>
          `${item.status === 'not_checkable' ? 'Please check: ' : ''}${item.concern} ${item.note}`,
      ) ?? [];
  const suggestions = (improvements.length ? improvements.map((item) => item.suggestion) : concerns)
    .filter(concise)
    .slice(0, 3);
  const directions = (round.research_suggestions ?? [])
    .filter((item) => ['strengthen', 'generalize'].includes(item.kind) && concise(item.suggestion))
    .slice(0, 2)
    .map((item) => item.suggestion);
  const authored = authoredSections(round, letter);
  const shortSection = (index: number) =>
    authored?.[index] && authored[index]!.split(/\s+/).length <= (index === 1 ? 160 : 130)
      ? authored[index]
      : undefined;
  const improvementText = suggestions.length
    ? suggestions.map((text) => `- ${text}`).join('\n\n')
    : (shortSection(1) ??
      (improvements.length || concerns.length
        ? 'The detailed report contains recommendations that need their full mathematical context.'
        : undefined));
  const directionText = directions.length
    ? directions.map((text) => `- ${text}`).join('\n\n')
    : shortSection(2);
  const candidates = formalizationAdvice(s, round);
  const states = [
    ['verified', 'with a verified formal proof'],
    ['in_progress', 'with saved unfinished proof work'],
    ['approved', 'approved for formalization'],
    ['declined', 'declined by the author'],
  ] as const;
  const progress = states.flatMap(([state, label]) => {
    const count = candidates.filter(
      (candidate) => candidate.state === state && candidate.progress !== 'blocked',
    ).length;
    return count ? [`${count} ${label}`] : [];
  });
  const blocked = candidates.filter((candidate) => candidate.progress === 'blocked').length;
  if (blocked) progress.push(`${blocked} with blocked formalization requiring review`);
  const formal = candidates.length
    ? `${candidates.length === 1 ? 'One result is' : `${candidates.length} results are`} recommended for formalization. ${progress.length ? `Current status: ${progress.join('; ')}. ` : ''}${s.purpose === 'review_only' ? 'This private review does not schedule formalization.' : candidates.some((candidate) => !candidate.state || candidate.state === 'proposed') ? 'Remaining recommendations require author approval before public formalization.' : ''} The targets and their scope are in the detailed report.`
    : 'No formalization is recommended in the available review.';
  const reasons =
    s.status.state === 'not_accepted' && s.decision?.decision === 'not_accepted'
      ? s.decision.reasons.map((reason) => `- ${publicationReason(reason, s)}`).join('\n\n')
      : '';
  return manuscriptReferences(
    [
      opening,
      reasons ? `## Reasons for the decision\n\n${reasons}` : '',
      overview(s, round, letter),
      improvementText ? `## Suggestions\n\n${improvementText}` : '',
      directionText
        ? `## Further directions\n\nThese are proposed extensions, not established results.\n\n${directionText}`
        : '',
      `## Formalization\n\n${formal}`,
    ]
      .filter(Boolean)
      .join('\n\n'),
    s.claims,
  );
}
