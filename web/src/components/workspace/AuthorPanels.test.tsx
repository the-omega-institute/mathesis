import { render, screen } from '@testing-library/react';
import { expect, it } from 'vitest';
import { setLocale } from '../../i18n/locale';
import { checkedTarget } from '../../test/referee';
import { ImprovementTargetDetails } from './AuthorPanels';

it.each([
  ['known', 'Known target'],
  ['specialization_of', 'Special case of an opened result'],
  ['related', 'Related result'],
  ['not_located', 'No matching statement located in the reported search'],
] as const)('renders the target prior-work finding %s faithfully', (verdict, label) => {
  render(
    <ImprovementTargetDetails
      item={{ ...checkedTarget, prior_work: { ...checkedTarget.prior_work!, verdict } }}
    />,
  );
  expect(screen.getByText(label)).toBeVisible();
  expect(screen.getByText(checkedTarget.prior_work!.note)).toBeVisible();
});

it('renders Chinese labels while preserving the mathematical text and source', () => {
  setLocale('zh');
  render(<ImprovementTargetDetails item={checkedTarget} />);
  for (const label of [
    '优先次序：',
    '目标命题：',
    '可行性：',
    '风险：',
    '该目标的已有研究：',
    '相关结果',
  ])
    expect(screen.getByText(label)).toBeVisible();
  expect(screen.getByText(checkedTarget.feasibility!)).toBeVisible();
  expect(screen.getByRole('link')).toHaveAttribute('href', 'https://example.org/finite-bound');
});

it('does not add target labels to legacy improvements', () => {
  const { container } = render(<ImprovementTargetDetails item={{}} />);
  expect(container).toBeEmptyDOMElement();
});
