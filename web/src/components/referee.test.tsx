import { act, fireEvent, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { ApiError, type ApiClient } from '../api/client';
import type { Person, RefereeRound, Submission } from '../api/types';
import { useReferee } from '../api/useReferee';
import { SubmissionDetailPage } from '../pages/SubmissionDetail';
import {
  accepted,
  analysis,
  author,
  contributor,
  editor,
  notAccepted,
  submission,
  version,
} from '../test/fixtures';
import { KATEX_ERROR, MACROS } from '../test/katex';
import {
  feedbackLetter,
  queuedRound,
  refereeAdvice,
  refereeAudit,
  refereeFile,
  refereeReport,
  refereeRound,
} from '../test/referee';
import { fakeApi, renderWithApp } from '../test/render';
import { RefereeWorkspace } from './referee';

const reviewer: Person = { ...editor, id: 'nyx|reviewer', roles: ['reviewer'] };

function roundWithFindings(concerns: typeof refereeAudit.concerns): RefereeRound {
  return {
    ...refereeRound,
    audit: {
      attempts: 1,
      state: {
        state: 'done',
        at: refereeRound.started_at,
        result: { ...refereeAudit, concerns },
      },
    },
  };
}

function workspace(overrides: Partial<ApiClient> = {}, person = editor, paper = submission) {
  const api = fakeApi(
    {
      getSubmission: vi.fn(async () => ({ ...paper, versions: [version(1, { macros: MACROS })] })),
      getAnalysis: vi.fn(async () => analysis),
      previewDecision: vi.fn(async () => paper.decision!),
      referee: vi.fn(async () => refereeFile()),
      ...overrides,
    },
    person,
  );
  return {
    ...renderWithApp(
      <SubmissionDetailPage
        id={paper.id}
        staffView={person.roles.some((role) => ['admin', 'editor', 'reviewer'].includes(role))}
      />,
      api,
    ),
    api,
  };
}

async function staffPanel() {
  const panel = (await screen.findByRole('heading', { name: 'Review', level: 2 })).closest(
    'section',
  )!;
  await userEvent.click(await within(panel).findByText(/^Current execution/));
  const details = screen.queryByText('Details: evidence, history and settings')?.closest('details');
  if (details && !details.open) await userEvent.click(details.querySelector('summary')!);
  await within(panel).findAllByRole('list', { name: 'Review progress' });
  return panel;
}

afterEach(() => {
  vi.useRealTimers();
});

describe('referee workspace', () => {
  it.each([
    undefined,
    queuedRound,
    { ...refereeRound, claims_revision: -1 },
    roundWithFindings([{ concern: 'Boundary case', status: 'confirmed', note: 'Fails at zero.' }]),
  ])('withholds intermediate reports before current delivery', async (round) => {
    workspace(
      { referee: vi.fn(async () => refereeFile({ rounds: round ? [round] : [], letters: [] })) },
      author,
    );
    expect(
      await screen.findByText(/The final report will appear here when the review is complete/),
    ).toBeVisible();
    expect(screen.queryByRole('region', { name: 'Final review report' })).toBeNull();
    expect(screen.queryByText('Respond to the review')).toBeNull();
    expect(screen.queryByText('Boundary case')).toBeNull();
  });

  it('shows one status line, the queued position, pending steps and the engine on hover', async () => {
    workspace({ referee: vi.fn(async () => refereeFile({ rounds: [queuedRound] })) });
    const panel = await staffPanel();
    expect(within(panel).getByRole('status')).toHaveTextContent(
      'Referee report: waiting in queue (#7).',
    );
    expect(within(panel).getByText(/In queue \(#7\)/)).toBeInTheDocument();
    expect(within(panel).getAllByText('Pending')).toHaveLength(4);
    expect(panel.querySelector('li[title="NyxID Oracle · ChatGPT Pro"]')).not.toBeNull();
    expect(within(panel).getByRole('button', { name: 'Start a new review' })).toBeDisabled();
    expect(within(panel).getByRole('list', { name: 'Review progress' })).toHaveTextContent(
      'Referee',
    );
  });

  it('shows a running step without a queue position', async () => {
    const round: RefereeRound = {
      ...queuedRound,
      referee: {
        ...queuedRound.referee,
        state: { state: 'running', since: refereeRound.started_at },
      },
    };
    workspace({ referee: vi.fn(async () => refereeFile({ rounds: [round] })) });
    const panel = await staffPanel();
    expect(within(panel).getByText(/Running · since/)).toBeInTheDocument();
    expect(within(panel).queryByText(/Waiting in queue/)).toBeNull();
  });

  it('renders the report as advisory, severity groups, readings, limits and the full answer with paper macros', async () => {
    workspace();
    const panel = await staffPanel();
    expect(within(panel).getAllByText('Done')).toHaveLength(5);
    await userEvent.click(within(panel).getByText('Referee report · minor revision'));
    expect(within(panel).getByText('Referee recommends minor revision')).toBeInTheDocument();
    expect(
      within(panel).getByText(/publication decision are recorded separately/),
    ).toBeInTheDocument();
    expect(within(panel).getByRole('heading', { name: 'Major concerns' })).toBeInTheDocument();
    expect(within(panel).getByRole('heading', { name: 'Minor concerns' })).toBeInTheDocument();
    expect(
      within(panel).getByRole('heading', { name: 'Limits of this review' }),
    ).toBeInTheDocument();
    const readings = within(panel).getByRole('table', {
      name: 'The referee’s per-statement readings',
    });
    expect(within(readings).getByText('Content')).toBeInTheDocument();
    expect(within(readings).getByText('Follows from known results')).toBeInTheDocument();
    expect(within(readings).getByText('A standard order argument.')).toBeInTheDocument();
    expect(within(readings).getByRole('link', { name: 'Theorem 1.1' })).toHaveAttribute(
      'href',
      '#ws-C1',
    );
    expect(document.getElementById('ws-C1')).toHaveAttribute('id', 'ws-C1');
    await userEvent.click(within(panel).getByText('Full referee answer'));
    expect(panel.querySelector('.mop')).toHaveTextContent('rep');
    expect(panel.querySelector(KATEX_ERROR)).toBeNull();
    expect(
      screen.getByRole('heading', { name: 'Publication decision', level: 2 }),
    ).toBeInTheDocument();
  });

  it('renders an unreadable recommendation without inventing one', async () => {
    const report = { ...refereeReport, recommendation: undefined };
    const round: RefereeRound = {
      ...refereeRound,
      referee: {
        ...refereeRound.referee,
        state: { state: 'done', result: report, at: refereeRound.started_at },
      },
    };
    workspace({ referee: vi.fn(async () => refereeFile({ rounds: [round] })) });
    const panel = await staffPanel();
    await userEvent.click(within(panel).getByText('Referee report · No recommendation'));
    expect(within(panel).getByText('No readable recommendation was returned.')).toBeInTheDocument();
    expect(within(panel).queryByText(/^Referee recommends/)).toBeNull();
  });

  it('shows failed and skipped reasons with expandable failure details', async () => {
    const round: RefereeRound = {
      ...refereeRound,
      referee: {
        attempts: 2,
        state: {
          state: 'failed',
          reason: 'Oracle unavailable',
          detail: 'The request timed out.',
          retryable: true,
          at: refereeRound.started_at,
        },
      },
      advice: { attempts: 0, state: { state: 'skipped', reason: 'No report to advise on' } },
      letter: { attempts: 0, state: { state: 'skipped', reason: 'No draft available' } },
    };
    workspace({ referee: vi.fn(async () => refereeFile({ rounds: [round] })) });
    const panel = await staffPanel();
    expect(within(panel).getByText('Failed: Oracle unavailable')).toBeInTheDocument();
    expect(within(panel).getByText('Skipped: No report to advise on')).toBeInTheDocument();
    await userEvent.click(within(panel).getByText('Failure details'));
    expect(within(panel).getByText('The request timed out.')).toBeInTheDocument();
    expect(within(panel).getByRole('button', { name: 'Start a new review' })).toBeEnabled();
  });

  it('distinguishes checked and proposed improvements and shows the checked evidence', async () => {
    workspace();
    const panel = await staffPanel();
    await userEvent.click(
      within(panel).getByText('Suggestions · 2 improvements, 1 statements that could go to Lean'),
    );
    const table = within(panel).getByRole('table', { name: 'Suggested improvements' });
    expect(within(table).getByText('Checked', { selector: '.badge' })).toHaveClass('badge-good');
    expect(within(table).getByText('Proposed', { selector: '.badge' })).toHaveClass('badge-warn');
    expect(within(table).getByText('For n = 1 both sides equal 1.')).toBeInTheDocument();
    expect(within(table).getByText('Whole paper')).toBeInTheDocument();
    expect(within(panel).getByText('Ready', { selector: '.badge' })).toBeInTheDocument();
    expect(within(panel).getByText('A wrapper lemma.')).toBeInTheDocument();
    expect(panel.querySelector('.code-block pre')).toHaveTextContent('theorem square_injective');
    expect(within(panel).queryByRole('button', { name: 'Propose formalization' })).toBeNull();
  });

  it('shows the Lean probe as private, with outcome, axioms, note and source', async () => {
    workspace();
    const panel = await staffPanel();
    await userEvent.click(within(panel).getByText('Lean check · 1 of 1 proved'));
    const probe = within(panel)
      .getByRole('heading', { name: 'Lean formalization probe' })
      .closest('div')!;
    expect(within(probe).getByText('Compiled in Lean', { selector: '.badge' })).toHaveClass(
      'badge-good',
    );
    expect(within(probe).getByText(/^Private ·/)).toHaveTextContent('Mathlib v4.33.0');
    expect(
      within(probe).getByText('Axioms: propext, Classical.choice, Quot.sound'),
    ).toBeInTheDocument();
    expect(within(probe).getByText('Mathesis.C2.main')).toBeInTheDocument();
    expect(within(probe).getByText('Lean source')).toBeInTheDocument();
    await userEvent.click(within(probe).getByText('Lean source'));
    expect(probe.querySelector('.code-block pre')).toHaveTextContent('import Mathlib');
    expect(within(panel).getByText('Lean check')).toBeInTheDocument();
  });

  it('posts the edited subject, body and note and displays the returned sent letter', async () => {
    const sendFeedback = vi.fn(async () => feedbackLetter);
    workspace({ sendFeedback });
    const panel = await staffPanel();
    const form = within(panel).getByRole('form', { name: 'Send feedback to the author' });
    expect(within(form).getByLabelText('Assessment')).toHaveDisplayValue('Minor revision');
    const user = userEvent.setup();
    const fields = {
      Subject: 'Revised editorial feedback',
      Body: 'A **checked** result: $\\rep$.',
      Note: 'A note using $\\code{x}$.',
    };
    for (const [label, value] of Object.entries(fields)) {
      await user.clear(within(form).getByLabelText(label));
      await user.paste(value);
    }
    await user.click(within(form).getByText('Preview', { selector: 'summary' }));
    const preview = within(form).getByRole('region', { name: 'Feedback preview' });
    expect(within(preview).getByText('checked', { selector: 'strong' })).toBeInTheDocument();
    expect(preview.querySelector('.mop')).toHaveTextContent('rep');
    expect(preview.querySelector(KATEX_ERROR)).toBeNull();
    await user.click(within(form).getByRole('button', { name: 'Send to author' }));
    expect(sendFeedback).toHaveBeenCalledExactlyOnceWith(submission.id, {
      subject: fields.Subject,
      body: fields.Body,
      note: fields.Note,
      assessment: 'minor_revision',
    });
    expect(
      await within(panel).findAllByRole('heading', { name: /Sent editorial feedback/, level: 3 }),
    ).not.toHaveLength(0);
    expect(within(panel).getByText(`Sent by ${editor.id}`)).toBeInTheDocument();
    expect(within(panel).getByText('Edited', { selector: '.badge' })).toBeInTheDocument();
    expect(within(panel).queryByRole('form', { name: 'Send feedback to the author' })).toBeNull();
    expect(within(panel).getAllByRole('status')[0]).toHaveTextContent(
      'Feedback sent to the authors on',
    );
  });

  it('warns on an empty body and preserves the draft when sending fails', async () => {
    const sendFeedback = vi.fn(async () => {
      throw new ApiError({ status: 409, code: 'conflict', detail: 'The current draft changed.' });
    });
    workspace({ sendFeedback });
    const panel = await staffPanel();
    const user = userEvent.setup();
    await user.clear(within(panel).getByLabelText('Body'));
    await user.click(within(panel).getByRole('button', { name: 'Send to author' }));
    expect(within(panel).getByRole('alert')).toHaveTextContent('The letter body is empty.');
    expect(sendFeedback).not.toHaveBeenCalled();
    await user.type(within(panel).getByLabelText('Body'), 'Specific feedback.');
    await user.click(within(panel).getByRole('button', { name: 'Send to author' }));
    expect(await within(panel).findByRole('alert')).toHaveTextContent('The current draft changed.');
    expect(within(panel).getByLabelText('Body')).toHaveValue('Specific feedback.');
    expect(within(panel).queryByText('Feedback sent to the author.')).toBeNull();
  });

  it.each(['accept', 'minor_revision', 'major_revision', 'reject', undefined] as const)(
    'defaults the letter assessment to the recommendation %s',
    async (recommendation) => {
      workspace({
        referee: vi.fn(async () =>
          refereeFile({
            rounds: [
              {
                ...refereeRound,
                referee: {
                  ...refereeRound.referee,
                  state: {
                    state: 'done',
                    result: { ...refereeReport, recommendation },
                    at: refereeRound.started_at,
                  },
                },
              },
            ],
          }),
        ),
      });
      const select = within(await staffPanel()).getByLabelText('Assessment');
      expect(select).toHaveValue(recommendation ?? '');
      expect(
        within(select)
          .getAllByRole('option')
          .map((option) => option.textContent),
      ).toEqual(['Accept', 'Minor revision', 'Major revision', 'Reject', 'No assessment']);
    },
  );

  it.each(['reject', ''] as const)(
    'sends the editor’s chosen assessment %s',
    async (assessment) => {
      const sendFeedback = vi.fn(async () => ({
        ...feedbackLetter,
        assessment: assessment || undefined,
      }));
      workspace({ sendFeedback });
      const form = within(await staffPanel()).getByRole('form', {
        name: 'Send feedback to the author',
      });
      await userEvent.selectOptions(within(form).getByLabelText('Assessment'), assessment);
      await userEvent.click(within(form).getByRole('button', { name: 'Send to author' }));
      expect(sendFeedback).toHaveBeenCalledExactlyOnceWith(submission.id, {
        subject:
          refereeRound.letter.state.state === 'done'
            ? refereeRound.letter.state.result.subject
            : '',
        body:
          refereeRound.letter.state.state === 'done' ? refereeRound.letter.state.result.body : '',
        note:
          refereeRound.letter.state.state === 'done' ? refereeRound.letter.state.result.note : '',
        ...(assessment ? { assessment } : {}),
      });
    },
  );

  it('keeps internal revision recommendations out of the author report', async () => {
    workspace(
      {
        referee: vi.fn(async () =>
          refereeFile({ letters: [{ ...feedbackLetter, assessment: 'minor_revision' }] }),
        ),
      },
      author,
      { ...submission, status: accepted.status, decision: accepted.decision },
    );
    const report = await screen.findByRole('region', { name: 'Final review report' });
    expect(within(report).getByRole('heading', { name: 'Review report' })).toBeVisible();
    expect(screen.queryByText('Minor revision')).toBeNull();
  });

  it('shows assessments in the staff’s earlier-letter history', async () => {
    workspace({
      referee: vi.fn(async () =>
        refereeFile({ letters: [{ ...feedbackLetter, round: undefined, assessment: 'reject' }] }),
      ),
    });
    const panel = await staffPanel();
    await userEvent.click(within(panel).getByText(/^Previous executions/));
    const badge = within(panel).getByText('Reject', { selector: '.badge' });
    expect(badge.closest('summary')).toHaveTextContent(feedbackLetter.subject);
  });

  it('shows one final report and keeps raw historical letters in staff history', async () => {
    workspace(
      {
        referee: vi.fn(async () =>
          refereeFile({
            letters: [
              { ...feedbackLetter, subject: 'Earlier feedback', sent_at: '2026-10-07T12:00:00Z' },
              feedbackLetter,
            ],
          }),
        ),
      },
      author,
      { ...submission, status: accepted.status, decision: accepted.decision },
    );
    const report = await screen.findByRole('region', { name: 'Final review report' });
    expect(within(report).getByRole('heading', { name: 'Review report' })).toBeVisible();
    expect(within(report).queryByText('Earlier feedback')).toBeNull();
    expect(report.querySelector(KATEX_ERROR)).toBeNull();
    await userEvent.click(screen.getByText('Manuscript details'));
    expect(screen.queryByText('Earlier reports')).toBeNull();
    expect(screen.queryByRole('heading', { name: 'Earlier feedback' })).toBeNull();
    expect(screen.queryByRole('form', { name: 'Send feedback to the author' })).toBeNull();
  });

  it('keeps the final report unavailable until a letter is delivered', async () => {
    const referee = vi.fn(async () => refereeFile());
    workspace({ referee }, author);
    await waitFor(() => {
      expect(referee).toHaveBeenCalledOnce();
      expect(screen.getByRole('region', { name: 'Paper standing' })).toBeInTheDocument();
    });
    expect(screen.queryByText('Full referee report', { selector: 'summary' })).toBeNull();
    expect(screen.queryByText('No feedback letters have been sent.')).toBeNull();
  });

  it('shows the complete contract-three mathematical sections with one download', async () => {
    workspace(
      {
        referee: vi.fn(async () =>
          refereeFile({
            rounds: [{ ...refereeRound, letter_contract: 3 }],
            letters: [
              {
                ...feedbackLetter,
                body: 'Dear authors,\n\nYour paper is accepted as WP-2026-0001.\n\n## Review analysis\n\nWe checked the induction step for $\\rep$.\n\n## Suggestions\n\nPlease clarify the boundary case.\n\n## Further directions\n\nStudy the same identity for infinite sequences.',
                note: 'A detailed mathematical argument.',
              },
            ],
          }),
        ),
      },
      author,
      { ...submission, status: accepted.status, decision: accepted.decision },
    );
    const panel = await screen.findByRole('region', { name: 'Final review report' });
    const details = within(panel)
      .getByText('Detailed report: findings and recommendations')
      .closest('details')!;
    expect(details).not.toHaveAttribute('open');
    expect(within(details).getByText('A detailed mathematical argument.')).not.toBeVisible();
    await userEvent.click(
      within(details).getByText('Detailed report: findings and recommendations'),
    );
    expect(within(details).getByText('Please clarify the boundary case.')).toBeVisible();
    expect(
      within(details).getByText('Study the same identity for infinite sequences.'),
    ).toBeVisible();
    expect(within(panel).getByText('A detailed mathematical argument.')).toBeVisible();
    expect(panel.querySelector('.mop')).toHaveTextContent('rep');
    expect(panel.querySelector(KATEX_ERROR)).toBeNull();
    expect(within(panel).getAllByRole('button', { name: 'Download report' })).toHaveLength(1);
    expect(within(panel).queryByText('Dear authors,')).toBeNull();
    expect(within(panel).queryByText(/WP-2026-0001/)).toBeNull();
  });

  it('presents legacy mathematical findings without the internal recommendation', async () => {
    workspace(
      {
        referee: vi.fn(async () =>
          refereeFile({
            letters: [
              {
                ...feedbackLetter,
                body: 'Status pending; recommendation: minor revision. Statement C1.',
                note: 'Internal status.',
              },
            ],
          }),
        ),
      },
      author,
      { ...submission, status: accepted.status, decision: accepted.decision },
    );
    const panel = await screen.findByRole('region', { name: 'Final review report' });
    expect(within(panel).getByText(refereeAudit.claims[0]!.comment)).not.toBeVisible();
    await userEvent.click(within(panel).getByText('Detailed report: findings and recommendations'));
    expect(within(panel).getByText(refereeAudit.claims[0]!.comment)).toBeVisible();
    expect(panel).toHaveTextContent('Congratulations!');
    expect(panel).not.toHaveTextContent(
      /minor revision|Status pending|Statement C1|Internal status/,
    );
  });

  it('gives reviewers and admins the staff view, with mutations reserved for editors and admins', async () => {
    const view = workspace({}, reviewer);
    let panel = await staffPanel();
    await userEvent.click(within(panel).getByText('Referee report · minor revision'));
    expect(within(panel).getByRole('heading', { name: 'Referee report' })).toBeInTheDocument();
    expect(within(panel).queryByRole('button', { name: /referee round/ })).toBeNull();
    expect(within(panel).queryByRole('button', { name: 'Send to author' })).toBeNull();
    view.unmount();
    workspace({}, { ...editor, roles: ['admin'] });
    panel = await staffPanel();
    expect(within(panel).getByRole('button', { name: 'Send to author' })).toBeInTheDocument();
  });

  it('does not request referee data for a non-author without a staff role', async () => {
    const referee = vi.fn(async () => refereeFile());
    workspace({ referee }, contributor);
    await screen.findByRole('heading', { name: submission.title });
    expect(referee).not.toHaveBeenCalled();
  });

  it('starts an absent round and restarts only a settled paper in review', async () => {
    const restartReferee = vi.fn(async () => refereeFile({ rounds: [queuedRound] }));
    const view = workspace({
      referee: vi.fn(async () => refereeFile({ rounds: [] })),
      restartReferee,
    });
    await userEvent.click(await screen.findByRole('button', { name: 'Start review' }));
    expect(restartReferee).toHaveBeenCalledExactlyOnceWith(submission.id);
    expect(await screen.findByRole('button', { name: 'Start a new review' })).toBeDisabled();
    view.unmount();
    workspace({}, editor, accepted);
    expect(
      within(await staffPanel()).getByRole('button', { name: 'Start a new review' }),
    ).toBeDisabled();
  });

  it('restarts a settled round and shows a conflict without losing the current report', async () => {
    const restartReferee = vi.fn(async () => {
      throw new ApiError({ status: 409, code: 'conflict', detail: 'A round is already active.' });
    });
    workspace({ restartReferee });
    const panel = await staffPanel();
    await userEvent.click(within(panel).getByRole('button', { name: 'Start a new review' }));
    expect(restartReferee).toHaveBeenCalledExactlyOnceWith(submission.id);
    expect(await within(panel).findByRole('alert')).toHaveTextContent('A round is already active.');
    await userEvent.click(within(panel).getByText('Referee report · minor revision'));
    expect(within(panel).getByRole('heading', { name: 'Referee report' })).toBeInTheDocument();
  });

  it('retains earlier rounds in expandable history, without an old send form', async () => {
    workspace({
      referee: vi.fn(async () =>
        refereeFile({
          rounds: [
            { ...refereeRound, number: 0, version: 0 },
            { ...queuedRound, number: 2 },
          ],
        }),
      ),
    });
    const panel = (await screen.findByRole('heading', { name: 'Review' })).closest('section')!;
    await userEvent.click(await within(panel).findByText(/^Current execution/));
    const details = screen
      .queryByText('Details: evidence, history and settings')
      ?.closest('details');
    if (details && !details.open) await userEvent.click(details.querySelector('summary')!);
    await within(panel).findAllByRole('list', { name: 'Review progress' });
    await userEvent.click(within(panel).getByText(/^Previous executions/));
    await userEvent.click(within(panel).getByText('Referee report · minor revision'));
    expect(within(panel).getByRole('heading', { name: 'Referee report' })).toBeInTheDocument();
    expect(within(panel).queryByRole('form', { name: 'Send feedback to the author' })).toBeNull();
  });

  it('proposes an eligible accepted statement with the candidate plan through the existing endpoint', async () => {
    const proposed: Submission = {
      ...accepted,
      formalization: {
        ...accepted.formalization,
        items: [
          ...accepted.formalization.items,
          {
            claim: 'C2',
            reason: refereeAdvice.formalization[0]!.plan,
            updated_at: feedbackLetter.sent_at,
            state: { state: 'proposed' },
          },
        ],
      },
    };
    const proposeFormalization = vi.fn(async () => proposed);
    workspace({ proposeFormalization }, editor, accepted);
    const panel = await staffPanel();
    await userEvent.click(
      within(panel).getByText('Suggestions · 2 improvements, 1 statements that could go to Lean'),
    );
    await userEvent.click(within(panel).getByRole('button', { name: 'Propose formalization' }));
    expect(proposeFormalization).toHaveBeenCalledExactlyOnceWith(
      accepted.id,
      'C2',
      refereeAdvice.formalization[0]!.plan,
    );
    await waitFor(() =>
      expect(within(panel).queryByRole('button', { name: 'Propose formalization' })).toBeNull(),
    );
  });

  it('hides formalization actions for reviewers, existing proposals, conjectures and unknown statements', async () => {
    const view = workspace({}, reviewer, accepted);
    let panel = await staffPanel();
    await userEvent.click(
      within(panel).getByText('Suggestions · 2 improvements, 1 statements that could go to Lean'),
    );
    expect(within(panel).queryByRole('button', { name: 'Propose formalization' })).toBeNull();
    view.unmount();
    const candidate = refereeAdvice.formalization[0]!;
    const advice = {
      ...refereeAdvice,
      formalization: ['C1', 'C3', 'missing'].map((claim) => ({ ...candidate, claim })),
    };
    const round: RefereeRound = {
      ...refereeRound,
      advice: {
        ...refereeRound.advice,
        state: { state: 'done', result: advice, at: refereeRound.started_at },
      },
    };
    workspace({ referee: vi.fn(async () => refereeFile({ rounds: [round] })) }, editor, accepted);
    panel = await staffPanel();
    await userEvent.click(
      within(panel).getByText('Suggestions · 2 improvements, 3 statements that could go to Lean'),
    );
    expect(within(panel).queryByRole('button', { name: 'Propose formalization' })).toBeNull();
  });

  it('puts the editable letter after progress and keeps evidence and preview closed', async () => {
    workspace();
    const panel = await staffPanel();
    const progress = within(panel).getByRole('list', { name: 'Review progress' });
    const letter = progress.nextElementSibling!;
    expect(within(letter as HTMLElement).getByRole('form')).toHaveAccessibleName(
      'Send feedback to the author',
    );
    for (const summary of [
      'Preview',
      'Referee report · minor revision',
      'Suggestions · 2 improvements, 1 statements that could go to Lean',
      'Lean check · 1 of 1 proved',
    ]) {
      expect(
        within(panel).getByText(summary, { selector: 'summary' }).closest('details'),
      ).not.toHaveAttribute('open');
    }
    expect(within(panel).getByLabelText('Body')).toBeVisible();
    expect(within(panel).getByRole('button', { name: 'Send to author' })).toBeVisible();
    expect(within(panel).getByRole('heading', { name: 'Referee report' })).not.toBeVisible();
    expect(within(panel).getByRole('region', { name: 'Feedback preview' })).not.toBeVisible();
  });

  it('omits the note preview when the note is empty', async () => {
    workspace();
    const panel = await staffPanel();
    await userEvent.clear(within(panel).getByLabelText('Note'));
    await userEvent.click(within(panel).getByText('Preview', { selector: 'summary' }));
    const preview = within(panel).getByRole('region', { name: 'Feedback preview' });
    expect(within(preview).getByRole('heading', { name: 'Body preview' })).toBeInTheDocument();
    expect(within(preview).queryByRole('heading', { name: 'Note preview' })).toBeNull();
  });
});

function PollingWorkspace({ paper, isStaff }: { paper: Submission; isStaff: boolean }) {
  const resource = useReferee(paper, isStaff);
  return (
    <RefereeWorkspace
      submission={paper}
      isStaff={isStaff}
      isEditor={isStaff}
      onChange={() => {}}
      resource={resource}
    />
  );
}

function polling(referee: ApiClient['referee'], isStaff = true, paper = submission) {
  const api = fakeApi({ referee });
  return renderWithApp(<PollingWorkspace paper={paper} isStaff={isStaff} />, api);
}

describe('referee polling', () => {
  it('polls every 30 seconds and stops once all steps settle', async () => {
    vi.useFakeTimers();
    const referee = vi
      .fn<ApiClient['referee']>()
      .mockResolvedValueOnce(refereeFile({ rounds: [queuedRound] }))
      .mockResolvedValue(refereeFile());
    polling(referee);
    await act(async () => {});
    expect(referee).toHaveBeenCalledTimes(1);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(29_999);
    });
    expect(referee).toHaveBeenCalledTimes(1);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1);
    });
    expect(referee).toHaveBeenCalledTimes(2);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(90_000);
    });
    expect(referee).toHaveBeenCalledTimes(2);
  });

  it('continues polling while advice is pending even if the referee report is done', async () => {
    vi.useFakeTimers();
    const referee = vi.fn(async () =>
      refereeFile({ rounds: [{ ...refereeRound, advice: queuedRound.advice }] }),
    );
    polling(referee);
    await act(async () => {});
    await act(async () => {
      await vi.advanceTimersByTimeAsync(30_000);
    });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(30_000);
    });
    expect(referee).toHaveBeenCalledTimes(3);
  });

  it('clears polling and aborts the request on unmount', async () => {
    vi.useFakeTimers();
    const referee = vi.fn(async () => refereeFile({ rounds: [queuedRound] }));
    const view = polling(referee);
    await act(async () => {});
    const signal = (referee.mock.calls[0] as unknown as Parameters<ApiClient['referee']>)[1]!
      .signal!;
    view.unmount();
    expect(signal.aborted).toBe(true);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(90_000);
    });
    expect(referee).toHaveBeenCalledTimes(1);
  });

  it('retries polling after a temporary fetch failure and stops when the next response settles', async () => {
    vi.useFakeTimers();
    const referee = vi
      .fn<ApiClient['referee']>()
      .mockResolvedValueOnce(refereeFile({ rounds: [queuedRound] }))
      .mockRejectedValueOnce(new Error('Temporary connection failure'))
      .mockResolvedValue(refereeFile());
    polling(referee);
    await act(async () => {});
    await act(async () => {
      await vi.advanceTimersByTimeAsync(30_000);
    });
    expect(screen.getByRole('alert')).toHaveTextContent('Temporary connection failure');
    await act(async () => {
      await vi.advanceTimersByTimeAsync(30_000);
    });
    expect(referee).toHaveBeenCalledTimes(3);
    expect(screen.queryByRole('alert')).toBeNull();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(60_000);
    });
    expect(referee).toHaveBeenCalledTimes(3);
  });

  it('preserves edited letter fields when a polling response arrives', async () => {
    vi.useFakeTimers();
    const referee = vi.fn(async () =>
      refereeFile({ rounds: [{ ...refereeRound, advice: queuedRound.advice }] }),
    );
    polling(referee);
    await act(async () => {});
    fireEvent.change(screen.getByLabelText('Subject'), { target: { value: 'Editor’s changes' } });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(30_000);
    });
    expect(referee).toHaveBeenCalledTimes(2);
    expect(screen.getByLabelText('Subject')).toHaveValue('Editor’s changes');
  });

  it('does not poll settled failures', async () => {
    vi.useFakeTimers();
    const failed: RefereeRound = {
      ...refereeRound,
      referee: {
        attempts: 1,
        state: {
          state: 'failed',
          reason: 'Unavailable',
          retryable: true,
          at: refereeRound.started_at,
        },
      },
    };
    const referee = vi.fn(async () => refereeFile({ rounds: [failed] }));
    polling(referee);
    await act(async () => {});
    await act(async () => {
      await vi.advanceTimersByTimeAsync(60_000);
    });
    expect(referee).toHaveBeenCalledTimes(1);
  });

  it('uses the latest round for polling and restart even when an older round is active', async () => {
    vi.useFakeTimers();
    const referee = vi.fn(async () =>
      refereeFile({ rounds: [{ ...refereeRound, number: 2 }, queuedRound] }),
    );
    polling(referee);
    await act(async () => {});
    expect(screen.getByRole('status')).toHaveTextContent('The feedback letter is ready.');
    expect(screen.getByRole('button', { name: 'Start a new review' })).toBeEnabled();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(60_000);
    });
    expect(referee).toHaveBeenCalledTimes(1);
  });

  it('polls and disables restart when the latest round is active despite settled history', async () => {
    vi.useFakeTimers();
    const referee = vi.fn(async () =>
      refereeFile({ rounds: [refereeRound, { ...queuedRound, number: 2 }] }),
    );
    polling(referee);
    await act(async () => {});
    expect(screen.getByRole('status')).toHaveTextContent('Referee report: waiting in queue (#7).');
    expect(screen.getByRole('button', { name: 'Start a new review' })).toBeDisabled();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(30_000);
    });
    expect(referee).toHaveBeenCalledTimes(2);
  });

  it.each([submission, accepted, notAccepted])(
    'polls with no letter section, then refreshes delivered letters on focus while $status.state',
    async (paper) => {
      vi.useFakeTimers();
      const referee = vi
        .fn<ApiClient['referee']>()
        .mockResolvedValueOnce(refereeFile({ rounds: [queuedRound] }))
        .mockResolvedValueOnce(refereeFile({ letters: [feedbackLetter] }))
        .mockResolvedValue(
          refereeFile({ letters: [{ ...feedbackLetter, subject: 'New feedback on focus' }] }),
        );
      const view = polling(referee, false, paper);
      await act(async () => {});
      if (paper.status.state === 'in_review') expect(view.container).toBeEmptyDOMElement();
      else expect(screen.getByRole('region', { name: 'Review' })).toBeInTheDocument();
      await act(async () => {
        await vi.advanceTimersByTimeAsync(29_999);
      });
      expect(referee).toHaveBeenCalledTimes(1);
      await act(async () => {
        await vi.advanceTimersByTimeAsync(1);
      });
      expect(referee).toHaveBeenCalledTimes(2);
      if (paper.status.state === 'in_review')
        expect(screen.queryByRole('region', { name: 'Final review report' })).toBeNull();
      else expect(screen.getByRole('region', { name: 'Final review report' })).toBeVisible();
      await act(async () => {
        fireEvent.focus(window);
      });
      expect(referee).toHaveBeenCalledTimes(3);
      expect(screen.queryByRole('heading', { name: 'New feedback on focus' })).toBeNull();
      expect(screen.queryByRole('heading', { name: 'Review' })).toBeNull();
      expect(screen.queryByText('Full referee report', { selector: 'summary' })).toBeNull();
      expect(screen.queryByRole('form', { name: 'Send feedback to the author' })).toBeNull();
    },
  );

  it('keeps refreshing an author view after initial and subsequent transient errors', async () => {
    vi.useFakeTimers();
    const referee = vi
      .fn<ApiClient['referee']>()
      .mockRejectedValueOnce(new Error('Temporary connection failure'))
      .mockResolvedValueOnce(refereeFile())
      .mockRejectedValueOnce(new Error('Another temporary failure'))
      .mockResolvedValue(refereeFile({ letters: [feedbackLetter] }));
    polling(referee, false, accepted);
    await act(async () => {});
    expect(screen.getByRole('alert')).toHaveTextContent('Temporary connection failure');
    await act(async () => {
      await vi.advanceTimersByTimeAsync(30_000);
    });
    expect(screen.queryByRole('alert')).toBeNull();
    expect(screen.getByRole('region', { name: 'Review' })).toBeInTheDocument();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(30_000);
    });
    expect(screen.getByRole('alert')).toHaveTextContent('Another temporary failure');
    await act(async () => {
      await vi.advanceTimersByTimeAsync(30_000);
    });
    expect(referee).toHaveBeenCalledTimes(4);
    expect(screen.queryByRole('alert')).toBeNull();
    expect(screen.getByRole('region', { name: 'Review' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Review report' })).toBeInTheDocument();
  });

  it('cleans up author polling, focus refresh and the request on unmount', async () => {
    vi.useFakeTimers();
    const referee = vi.fn<ApiClient['referee']>().mockResolvedValue(refereeFile());
    const view = polling(referee, false);
    await act(async () => {});
    const signal = referee.mock.calls[0]![1]!.signal!;
    view.unmount();
    expect(signal.aborted).toBe(true);
    await act(async () => {
      fireEvent.focus(window);
      await vi.advanceTimersByTimeAsync(60_000);
    });
    expect(referee).toHaveBeenCalledTimes(1);
  });

  it('does not poll or refresh on focus for an author with a draft paper', async () => {
    vi.useFakeTimers();
    const referee = vi.fn(async () => refereeFile({ rounds: [queuedRound] }));
    polling(referee, false, { ...submission, status: { state: 'draft' } });
    await act(async () => {});
    await act(async () => {
      fireEvent.focus(window);
      await vi.advanceTimersByTimeAsync(60_000);
    });
    expect(referee).toHaveBeenCalledTimes(1);
  });
});

it('shows private review completion without requesting a publication decision', async () => {
  const previewDecision = vi.fn();
  workspace({ previewDecision }, author, {
    ...submission,
    purpose: 'review_only',
    status: { state: 'review_complete' },
    decision: undefined,
  });
  expect(
    await screen.findByRole('heading', { name: 'Private review in progress' }),
  ).toBeInTheDocument();
  await userEvent.click(screen.getByText('Manuscript details'));
  expect(screen.queryByRole('button', { name: 'Download report' })).toBeNull();
  expect(screen.queryByRole('heading', { name: 'Publication decision' })).toBeNull();
  expect(previewDecision).not.toHaveBeenCalled();
  expect(screen.queryByText(/Volunteer contributors may help analyse/)).toBeNull();
  expect(screen.queryByText('Advanced: compute settings and history')).toBeNull();
});

it('does not mistake hidden author letter fields for a finished review without delivery', async () => {
  workspace(
    {
      referee: vi.fn(async () =>
        refereeFile({
          letters: [],
          rounds: [{ ...refereeRound, advice: undefined, letter: undefined }],
        }),
      ),
    },
    author,
  );
  const panel = await screen.findByRole('region', { name: 'Paper standing' });
  expect(
    await within(panel).findByText(/The final report will appear here when the review is complete/),
  ).toBeInTheDocument();
  expect(within(panel).queryByText('The review finished without a letter.')).toBeNull();
});

it('shows safe hidden-stage progress, survives a fetch error and stops polling after delivery', async () => {
  vi.useFakeTimers();
  const at = refereeRound.started_at;
  const progress = {
    referee: { state: 'done' as const, at },
    audit: { state: 'done' as const, at },
    advice: { state: 'running' as const, since: at },
    letter: { state: 'pending' as const },
    formal: { state: 'skipped' as const },
  };
  const running = { ...refereeRound, progress, advice: undefined, letter: undefined };
  const finished = {
    ...running,
    progress: {
      ...progress,
      advice: { state: 'done' as const, at },
      letter: { state: 'done' as const, at },
    },
  };
  const referee = vi
    .fn<ApiClient['referee']>()
    .mockResolvedValueOnce(refereeFile({ rounds: [running] }))
    .mockRejectedValueOnce(new Error('Connection interrupted'))
    .mockResolvedValue(refereeFile({ rounds: [finished], letters: [feedbackLetter] }));
  polling(referee, false);
  await act(async () => {});
  expect(screen.queryByRole('region', { name: 'Final review report' })).toBeNull();
  expect(screen.queryByRole('list', { name: 'Review stage details' })).toBeNull();
  expect(screen.queryByRole('form', { name: 'Send feedback to the author' })).toBeNull();
  await act(async () => {
    await vi.advanceTimersByTimeAsync(30_000);
  });
  expect(referee).toHaveBeenCalledTimes(2);
  await act(async () => {
    await vi.advanceTimersByTimeAsync(30_000);
  });
  expect(screen.queryByText('Available feedback — decision pending')).toBeNull();
  expect(screen.queryByRole('region', { name: 'Final review report' })).toBeNull();
  expect(screen.queryByRole('button', { name: 'Download report' })).toBeNull();
  await act(async () => {
    await vi.advanceTimersByTimeAsync(90_000);
  });
  expect(referee).toHaveBeenCalledTimes(3);
  await act(async () => {
    fireEvent.focus(window);
  });
  expect(referee).toHaveBeenCalledTimes(4);
});

it('shows a failed hidden letter without leaking its payload or claiming it is still being written', async () => {
  const at = refereeRound.started_at;
  workspace(
    {
      referee: vi.fn(async () =>
        refereeFile({
          rounds: [
            {
              ...refereeRound,
              advice: undefined,
              letter: undefined,
              progress: {
                referee: { state: 'done', at },
                audit: { state: 'done', at },
                advice: { state: 'done', at },
                letter: { state: 'failed', at, retryable: false },
                formal: { state: 'skipped' },
              },
            },
          ],
          letters: [],
        }),
      ),
    },
    author,
  );
  const panel = await screen.findByRole('region', { name: 'Paper standing' });
  expect(
    await within(panel).findByText(/The review needs attention before it can continue/),
  ).toBeVisible();
  expect(within(panel).queryByText(/has not been delivered yet/)).toBeNull();
});

it('keeps polling a saved response before the worker admits its next round', async () => {
  vi.useFakeTimers();
  const at = refereeRound.started_at;
  const progress = {
    referee: { state: 'done' as const, at },
    audit: { state: 'done' as const, at },
    advice: { state: 'done' as const, at },
    letter: { state: 'done' as const, at },
    formal: { state: 'skipped' as const },
  };
  const round = { ...refereeRound, progress };
  const response = {
    id: 'waiting-response',
    round: 1,
    concern: 0,
    version: round.version,
    claims_revision: round.claims_revision,
    body: 'Please recheck the supplied evidence.',
    author: author.id,
    at,
  };
  const referee = vi
    .fn<ApiClient['referee']>()
    .mockResolvedValueOnce(
      refereeFile({ rounds: [round], letters: [feedbackLetter], finding_responses: [response] }),
    )
    .mockResolvedValue(
      refereeFile({
        rounds: [
          round,
          {
            ...queuedRound,
            number: 2,
            response_ids: [response.id],
            progress: {
              ...progress,
              referee: { state: 'running', since: at },
              audit: { state: 'pending' },
              advice: { state: 'pending' },
              letter: { state: 'pending' },
            },
          },
        ],
        letters: [feedbackLetter],
        finding_responses: [response],
      }),
    );
  polling(referee, false, {
    ...submission,
    purpose: 'review_only',
    status: { state: 'review_complete' },
  });
  await act(async () => {});
  expect(screen.getByRole('status')).toHaveTextContent('A follow-up review is queued.');
  await act(async () => {
    await vi.advanceTimersByTimeAsync(30_000);
  });
  expect(referee).toHaveBeenCalledTimes(2);
  expect(screen.queryByRole('region', { name: 'Final review report' })).toBeNull();
  expect(screen.queryByRole('region', { name: 'Final review report' })).toBeNull();
});
