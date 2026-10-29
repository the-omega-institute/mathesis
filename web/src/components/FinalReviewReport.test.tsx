import { fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, it, vi } from 'vitest';
import { accepted, submission } from '../test/fixtures';
import { feedbackLetter, refereeFile, refereeRound } from '../test/referee';
import { authorReport } from '../lib/authorReport';
import { FinalReviewReport } from './FinalReviewReport';

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
    `# Mathesis review: ${accepted.title}\n\n${authorReport(accepted, refereeRound, feedbackLetter)}\n`,
  );
  expect(downloaded).not.toContain(feedbackLetter.subject);
});
