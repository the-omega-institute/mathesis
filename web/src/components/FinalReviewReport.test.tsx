import { fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, it, vi } from 'vitest';
import { accepted, submission } from '../test/fixtures';
import { feedbackLetter, refereeFile, refereeRound } from '../test/referee';
import { authorReport, authorReportPreview } from '../lib/authorReport';
import type { FormalizationProgressState, FormalizationWork } from '../api/types';
import { author } from '../test/fixtures';
import { fakeApi, renderWithApp } from '../test/render';
import { FinalReviewReport } from './FinalReviewReport';
import { ProofWork } from './ProofWork';

afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

it('never exposes an internal delivered letter as an in-review final report or download', () => {
  const { container } = render(
    <FinalReviewReport submission={submission} file={refereeFile({ letters: [feedbackLetter] })} />,
  );
  expect(container).toBeEmptyDOMElement();
});

it('downloads exactly the same outcome and mathematical report as the page', () => {
  vi.useFakeTimers();
  let downloaded = '';
  vi.stubGlobal(
    'Blob',
    class {
      constructor(parts: string[]) {
        downloaded = parts.join('');
      }
    },
  );
  const create = vi.fn(() => 'blob:review');
  Object.defineProperty(URL, 'createObjectURL', { configurable: true, value: create });
  const click = vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(() => {});
  render(
    <FinalReviewReport submission={accepted} file={refereeFile({ letters: [feedbackLetter] })} />,
  );
  fireEvent.click(screen.getByRole('button', { name: 'Download report' }));
  expect(click).toHaveBeenCalledOnce();
  expect(downloaded).toBe(
    `# Mathesis review: ${accepted.title}\n\n${authorReportPreview(accepted, refereeRound, feedbackLetter)}\n\n## Detailed report\n\n${authorReport(accepted, refereeRound, feedbackLetter)}\n`,
  );
  expect(downloaded).not.toContain(feedbackLetter.subject);
});

it.each([
  ['paused', 'partial', 'Automatic proof work is paused. Saved work is retained.'],
  ['expired', 'in_progress', 'The allowance has expired. Saved work is retained.'],
  ['allowance_used', 'partial', 'The authorized allowance has been used. Saved work is retained.'],
  ['not_authorized', 'partial', 'Automatic proof work awaits an author allowance.'],
  ['completed', 'blocked', 'The final target has an independently checked result.'],
] as const)(
  'keeps the report and download truthful beside %s controls and %s saved work',
  async (state: FormalizationWork['state'], progress: FormalizationProgressState, label) => {
    const paper = {
      ...accepted,
      formalization: {
        items: [
          {
            claim: 'C1',
            reason: 'Formalize the compactness lemma.',
            state: { state: 'in_progress' as const },
            progress: {
              state: progress,
              attempts: 2,
              completed_lemmas: [],
              remaining_obligations: ['Review the saved independent result.'],
              updated_at: accepted.updated_at,
            },
            updated_at: accepted.updated_at,
          },
        ],
      },
    };
    let downloaded = '';
    vi.stubGlobal(
      'Blob',
      class {
        constructor(parts: string[]) {
          downloaded = parts.join('');
        }
      },
    );
    Object.defineProperty(URL, 'createObjectURL', {
      configurable: true,
      value: () => 'blob:review',
    });
    vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(() => {});
    renderWithApp(
      <>
        <FinalReviewReport submission={paper} file={refereeFile({ letters: [feedbackLetter] })} />
        <ProofWork record="WP-2026-0001" claim="C1" />
      </>,
      fakeApi(
        {
          formalizationWork: vi.fn(async () => ({
            target_digest: 'internal-pin',
            total_generations: 2,
            used_generations: 2,
            expires_at: '2026-11-01T00:00:00Z',
            paused: state === 'paused',
            state,
            can_control: true,
            managed: true,
          })),
          checkedContributions: vi.fn(async () => []),
        },
        author,
      ),
    );
    expect(await screen.findByText(label)).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Download report' }));
    expect(downloaded).not.toMatch(/will continue|being formalized|Proof work is in progress/i);
    expect(
      screen.queryByText(/will continue|being formalized|Proof work is in progress/i),
    ).toBeNull();
    if (progress === 'blocked') {
      expect(downloaded).toContain('Formalization is blocked.');
      expect(downloaded).toContain('with blocked formalization requiring review');
    } else {
      expect(downloaded).toContain('Saved proof work is unfinished.');
      expect(downloaded).toContain('requires an active remaining author allowance');
      expect(downloaded).toContain('with saved unfinished proof work');
    }
    expect(downloaded).not.toContain('internal-pin');
  },
);
