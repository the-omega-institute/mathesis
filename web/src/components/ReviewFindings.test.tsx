import { act, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, it, vi } from 'vitest';
import { accepted, author, submission } from '../test/fixtures';
import { feedbackLetter, refereeAudit, refereeFile, refereeRound } from '../test/referee';
import { fakeApi, renderWithApp } from '../test/render';
import { ReviewFindings } from './ReviewFindings';

it('retains local evidence and binds an author response to the confirmed current source', async () => {
  const file = refereeFile({
    rounds: [
      {
        ...refereeRound,
        audit: {
          attempts: 1,
          state: {
            state: 'done',
            at: refereeRound.started_at,
            result: {
              ...refereeAudit,
              concerns: [
                {
                  concern: 'Boundary case',
                  status: 'confirmed',
                  note: 'The inequality fails.',
                  basis: 'offline',
                  evidence: 'At n = 0 the sides are 1 and 0.',
                  location: 'Lemma 2',
                },
              ],
            },
          },
        },
      },
    ],
  });
  const respondToFinding = vi.fn(async () => file);
  const onChange = vi.fn();
  renderWithApp(
    <ReviewFindings submission={submission} file={file} onChange={onChange} />,
    fakeApi({ respondToFinding }, author),
  );
  const user = userEvent.setup();
  await user.click(await screen.findByText('R1-A1'));
  expect(screen.getByText('At n = 0 the sides are 1 and 0.')).toBeInTheDocument();
  expect(screen.getByText('Reported location: Lemma 2')).toBeInTheDocument();
  expect(screen.getByText(/A response does not mark a finding resolved/)).toBeInTheDocument();
  await user.type(
    await screen.findByLabelText('Your response or correction'),
    'The domain is n > 0.',
  );
  await user.click(screen.getByRole('button', { name: 'Send response & request re-review' }));
  await waitFor(() => expect(onChange).toHaveBeenCalledWith(file));
  expect(respondToFinding).toHaveBeenCalledWith(submission.id, {
    round: 1,
    concern: 0,
    version: submission.versions.at(-1)!.number,
    claims_revision: submission.claims_revision,
    body: 'The domain is n > 0.',
  });
});

it('distinguishes re-reviewed responses from resolved findings', async () => {
  const response = {
    id: 'response1',
    round: 1,
    concern: 0,
    version: refereeRound.version,
    claims_revision: refereeRound.claims_revision,
    body: 'Here is the source.',
    author: author.id,
    at: refereeRound.started_at,
  };
  const file = refereeFile({
    finding_responses: [response],
    rounds: [refereeRound, { ...refereeRound, number: 2, response_ids: ['response1'] }],
  });
  renderWithApp(
    <ReviewFindings submission={submission} file={file} onChange={vi.fn()} />,
    fakeApi({}, author),
  );
  expect(
    await screen.findByText('Re-reviewed in round 2. Read that audit for the conclusion.'),
  ).toBeInTheDocument();
  expect(screen.queryByText('Resolved')).toBeNull();
});

it('shows the response conclusion with version-bound evidence without claiming proof verification', async () => {
  const response = {
    id: 'reply',
    round: 1,
    concern: 0,
    version: refereeRound.version,
    claims_revision: refereeRound.claims_revision,
    body: 'The attachment covers the zero case.',
    author: author.id,
    at: refereeRound.started_at,
  };
  const file = refereeFile({
    finding_responses: [response],
    rounds: [
      refereeRound,
      {
        ...refereeRound,
        number: 2,
        response_ids: ['reply'],
        audit: {
          attempts: 1,
          state: {
            state: 'done',
            at: refereeRound.started_at,
            result: {
              ...refereeAudit,
              response_assessments: [
                {
                  response_id: 'reply',
                  status: 'addressed',
                  explanation: 'The zero-case calculation gives equality.',
                  evidence: [{ path: 'evidence/cases 1.json', sha256: 'a'.repeat(64) }],
                },
              ],
            },
          },
        },
      },
    ],
  });
  renderWithApp(
    <ReviewFindings
      submission={{ ...submission, claims_revision: response.claims_revision }}
      file={file}
      onChange={vi.fn()}
    />,
    fakeApi({}, author),
  );
  await userEvent.click(await screen.findByText('R1-A1'));
  expect(screen.getByText('Addressed in this review')).toBeInTheDocument();
  expect(screen.getByText('The zero-case calculation gives equality.')).toBeInTheDocument();
  expect(screen.getByRole('link', { name: 'evidence/cases 1.json' })).toHaveAttribute(
    'href',
    `/api/v1/submissions/${encodeURIComponent(submission.id)}/evidence?version=${response.version}&path=evidence%2Fcases%201.json`,
  );
  expect(screen.getByText(/not an independent proof or execution receipt/)).toBeInTheDocument();
  expect(screen.queryByText('Resolved')).toBeNull();
});

it('keeps an older source conclusion visibly historical', async () => {
  const response = {
    id: 'old',
    round: 1,
    concern: 0,
    version: refereeRound.version,
    claims_revision: refereeRound.claims_revision,
    body: 'Old response',
    author: author.id,
    at: refereeRound.started_at,
  };
  const file = refereeFile({
    finding_responses: [response],
    rounds: [refereeRound, { ...refereeRound, number: 2, response_ids: ['old'] }],
  });
  renderWithApp(
    <ReviewFindings
      submission={{ ...submission, claims_revision: response.claims_revision + 1 }}
      file={file}
      onChange={vi.fn()}
    />,
    fakeApi({}, author),
  );
  await userEvent.click(await screen.findByText('R1-A1'));
  expect(
    screen.getByText('This response belongs to an earlier source or statement revision.'),
  ).toBeInTheDocument();
  expect(
    screen.queryByText('Re-reviewed in round 2. Read that audit for the conclusion.'),
  ).toBeNull();
});

it('keeps source identifiers out of the author-facing points', async () => {
  const file = refereeFile({
    letters: [feedbackLetter],
    rounds: [
      {
        ...refereeRound,
        source_sha256: 'b'.repeat(64),
        audit: {
          attempts: 1,
          state: {
            state: 'done',
            at: refereeRound.started_at,
            result: {
              ...refereeAudit,
              concerns: [
                {
                  concern: 'A boundary case needs clarification.',
                  status: 'confirmed',
                  note: 'The final sentence needs one more hypothesis.',
                  basis: 'offline',
                  evidence: 'The argument uses the hypothesis at this step.',
                },
              ],
            },
          },
        },
      },
    ],
  });
  renderWithApp(
    <ReviewFindings submission={accepted} file={file} onChange={vi.fn()} presentation="author" />,
    fakeApi({}, author),
  );

  const points = await screen.findByRole('region', { name: 'Points to address' });
  expect(points).toHaveTextContent('Points to address');
  expect(points).not.toHaveTextContent('Round 1');
  expect(points).not.toHaveTextContent('Source SHA-256');
  expect(points).not.toHaveTextContent('bbbbbbbb');
  expect(points).not.toHaveTextContent('R1-A1');
});

it('shows only findings from the latest delivered report, excluding refuted and unpublished findings', async () => {
  const withConcern = (
    number: number,
    concern: string,
    status: 'confirmed' | 'refuted' = 'confirmed',
  ) => ({
    ...refereeRound,
    number,
    audit: {
      attempts: 1,
      state: {
        state: 'done' as const,
        at: refereeRound.started_at,
        result: { ...refereeAudit, concerns: [{ concern, status, note: 'Review detail.' }] },
      },
    },
  });
  const file = refereeFile({
    rounds: [
      withConcern(1, 'Earlier concern'),
      withConcern(2, 'Current concern'),
      withConcern(3, 'Unpublished concern'),
    ],
    letters: [feedbackLetter, { ...feedbackLetter, round: 2 }],
  });
  const { unmount } = renderWithApp(
    <ReviewFindings submission={accepted} file={file} onChange={vi.fn()} presentation="author" />,
    fakeApi({}, author),
  );
  await waitFor(() => expect(screen.getByText('Current concern')).toBeInTheDocument());
  expect(screen.queryByText('Earlier concern')).toBeNull();
  expect(screen.queryByText('Unpublished concern')).toBeNull();
  unmount();
  await act(async () => {
    renderWithApp(
      <ReviewFindings
        submission={accepted}
        file={{ ...file, rounds: [withConcern(2, 'Refuted concern', 'refuted')] }}
        onChange={vi.fn()}
        presentation="author"
      />,
      fakeApi({}, author),
    );
  });
  expect(screen.queryByText('Refuted concern')).toBeNull();
});
