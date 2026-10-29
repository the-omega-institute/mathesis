import { describe, expect, it } from 'vitest';
import { accepted, notAccepted, submission } from '../test/fixtures';
import { feedbackLetter, refereeAudit, refereeRound } from '../test/referee';
import { authorReport, publicationReason, reportOpening } from './authorReport';
import { manuscriptReferences } from './statements';

describe('author publication reports', () => {
  it('explains contribution and significance decisions without internal policy vocabulary', () => {
    const overlap = publicationReason(
      {
        reason: 'insufficient_evidence',
        claim: 'C1',
        detail:
          'Statement C1 is covered by the source-audited pinned knowledge comparison and cannot support admission as a new result. Any claimed named open-problem settlement needs its independent audited evidence.',
      },
      submission,
    );
    expect(overlap).toBe(
      'Theorem 1.1 follows from the compared prior results. A claimed resolution of an open problem needs separate mathematical justification.',
    );
    const unclear = publicationReason(
      {
        reason: 'insufficient_evidence',
        claim: 'C1',
        detail:
          'Statement C1 has an undetermined proof-escape reading; further evidence is needed.',
      },
      submission,
    );
    expect(unclear).toContain('could not establish a distinct new mathematical contribution');
    expect(unclear).not.toContain('proof-escape');
    const utility = publicationReason(
      {
        reason: 'insufficient_utility',
        claim: 'C1',
        detail:
          'Statement C1 has no established independent utility under its mathematical or computational category: The finite sample does not distinguish the proposed bound from the known bound.',
      },
      submission,
    );
    expect(utility).toContain(
      'The finite sample does not distinguish the proposed bound from the known bound.',
    );
    expect(utility).not.toMatch(/C1|category|independent utility/);
  });
  it('withholds a pending report even when the letter recommends acceptance', () => {
    expect(
      authorReport(submission, refereeRound, { ...feedbackLetter, assessment: 'accept' }),
    ).toBeUndefined();
    expect(reportOpening({ ...submission, status: { state: 'withdrawn' } })).toBeUndefined();
  });

  it('uses the applied outcome, preserves the mathematical findings and separates research directions', () => {
    const report = authorReport(
      { ...accepted, analysis_visibility: 'public' },
      {
        ...refereeRound,
        research_suggestions: [
          {
            kind: 'literature',
            claim: 'C1',
            suggestion: 'Attribute C1 to the cited transposition bound.',
            excerpt: false,
          },
          {
            kind: 'generalize',
            suggestion:
              'Test whether the minimum-degree hypothesis can be replaced by a local obstruction condition; this remains unproved.',
            excerpt: false,
          },
        ],
      },
      {
        ...feedbackLetter,
        assessment: 'reject',
        body: 'Minor revision. C1 needs an audit.',
        note: 'Decision pending.',
      },
    )!;
    expect(report).toContain('Congratulations!');
    expect(report).toContain('now displayed on Mathesis');
    expect(report).toContain(refereeAudit.claims[0]!.comment);
    expect(report).toContain('Attribute Theorem 1.1 to the cited transposition bound.');
    expect(report.split('## Further directions')[1]).toContain('this remains unproved');
    expect(report).toContain('## Formalization opportunities');
    expect(report).not.toMatch(/C1|Minor revision|Decision pending|audit/);
  });

  it('explains actual refusals using the source result name and prior source', () => {
    const report = authorReport(notAccepted, refereeRound, {
      ...feedbackLetter,
      assessment: 'accept',
    })!;
    expect(report).toContain('We are sorry');
    expect(report).toContain('Theorem 1.1 already follows from 1901.00001');
    expect(report).not.toContain('Congratulations');
    expect(publicationReason({ reason: 'unchecked_result', claim: 'C1' }, submission)).toContain(
      'not a demonstrated mathematical error',
    );
  });

  it('respects private review and private accepted visibility', () => {
    const privateReport = authorReport(
      { ...submission, purpose: 'review_only', status: { state: 'review_complete' } },
      refereeRound,
      feedbackLetter,
    )!;
    expect(privateReport).toContain('no publication decision has been made');
    expect(privateReport).not.toMatch(/Congratulations|cannot accept|now displayed/);
    expect(reportOpening({ ...accepted, analysis_visibility: 'private' })).toContain(
      'remains private',
    );
  });

  it('keeps original theorem numbering and protects formulas, code and source links', () => {
    const text =
      'Statement C1 uses C2. C1–C3 are discussed. $C1 + C2$ `C1` https://example.org/C1 [source](./C1.tex) \\(C2\\)';
    const readable = manuscriptReferences(text, submission.claims);
    expect(readable).toContain('Theorem 1.1 uses Lemma 2.1');
    expect(readable).toContain('Theorem 1.1 through Conjecture 4.1');
    expect(readable).toContain('$C1 + C2$ `C1` https://example.org/C1 [source](./C1.tex) \\(C2\\)');
  });

  it('uses the new mathematical sections without the old salutation or decision sentence', () => {
    const report = authorReport(
      accepted,
      { ...refereeRound, letter_contract: 3 },
      {
        ...feedbackLetter,
        body: 'Dear authors,\n\nYour paper is accepted as WP-2026-0001.\n\n## Review analysis\n\nC1 holds under the finite-domain hypothesis.\n\n## Suggestions\n\nState that hypothesis explicitly.\n\n## Further directions\n\nDetermine whether compactness extends the theorem to infinite domains.',
        note: 'The compactness argument would require a uniform bound.',
      },
    )!;
    expect(report).toContain('Theorem 1.1 holds under the finite-domain hypothesis');
    expect(report).toContain('require a uniform bound');
    expect(report).not.toMatch(/Dear authors|WP-2026|C1/);
  });

  it('falls back to source findings when new sections are missing and omits refuted concerns', () => {
    const report = authorReport(
      accepted,
      {
        ...refereeRound,
        letter_contract: 3,
        audit: {
          attempts: 1,
          state: {
            state: 'done',
            at: '',
            result: {
              ...refereeAudit,
              concerns: [
                { concern: 'A disproved objection.', note: 'Resolved.', status: 'refuted' },
              ],
            },
          },
        },
      },
      { ...feedbackLetter, body: 'Minor revision required.' },
    )!;
    expect(report).toContain(refereeAudit.claims[0]!.comment);
    expect(report).not.toMatch(/Minor revision|disproved objection/);
  });
});
