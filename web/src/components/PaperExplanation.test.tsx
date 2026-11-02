import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { PaperExplanation } from './PaperExplanation';

describe('paper explanation', () => {
  it('switches between complete English and Chinese readings with language metadata', async () => {
    const user = userEvent.setup();
    render(
      <PaperExplanation
        text={{
          en: 'A familiar question.\n\nWhat the authors prove.',
          zh: '一个熟悉的问题。\n\n作者证明了什么。',
        }}
      />,
    );
    expect(screen.getByRole('heading', { name: 'In plain language' })).toBeInTheDocument();
    expect(screen.getByText('A familiar question.').parentElement).toHaveAttribute('lang', 'en');
    expect(screen.queryByText('一个熟悉的问题。')).not.toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: '中文' }));
    expect(screen.getByRole('heading', { name: '白话解读' })).toBeInTheDocument();
    expect(screen.getByText('作者证明了什么。').parentElement).toHaveAttribute('lang', 'zh-Hans');
    expect(screen.getByRole('button', { name: '中文' })).toHaveAttribute('aria-pressed', 'true');
    expect(screen.queryByText('A familiar question.')).not.toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'English' }));
    expect(screen.getByText('What the authors prove.')).toBeInTheDocument();
  });
  it('omits an absent or incomplete explanation instead of showing a placeholder', () => {
    const { container, rerender } = render(<PaperExplanation />);
    expect(container).toBeEmptyDOMElement();
    rerender(<PaperExplanation text={{ en: 'Only one language', zh: ' ' }} />);
    expect(container).toBeEmptyDOMElement();
  });
});
