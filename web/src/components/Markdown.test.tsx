import { render } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { currentMacros, normaliseMacros } from '../lib/macros';
import { KATEX_ERROR, MACROS } from '../test/katex';
import { LatexText, MacrosProvider, Markdown } from './Markdown';

describe('paper macros in KaTeX', () => {
  it('renders a preamble macro as an operator name instead of an error', () => {
    const { container } = render(
      <MacrosProvider macros={MACROS}>
        <LatexText source="Let $\rep(x) = \code{ab}$." />
      </MacrosProvider>,
    );
    expect(container.querySelector('.katex')).not.toBeNull();
    expect(container.querySelector(KATEX_ERROR)).toBeNull();
    expect(container.querySelector('.mop')).toHaveTextContent('rep');
    expect(container.querySelector('.texttt')).toHaveTextContent('ab');
  });

  it('shows an unknown command as an error without the macros', () => {
    const { container } = render(<LatexText source="Let $\rep(x)$." />);
    expect(container.querySelector(KATEX_ERROR)).not.toBeNull();
  });

  it('gives each render its own copy, so \\gdef does not leak', () => {
    const shared = { ...MACROS };
    const { container } = render(
      <MacrosProvider macros={shared}>
        <LatexText source="$\gdef\leak{x}\leak$" />
        <div className="second">
          <LatexText source="$\leak$" />
        </div>
      </MacrosProvider>,
    );
    expect(shared).toEqual(MACROS);
    expect(container.querySelector('.second')?.querySelector(KATEX_ERROR)).not.toBeNull();
  });

  it('treats missing or null macros as none', () => {
    expect(normaliseMacros(null)).toEqual({});
    expect(normaliseMacros(undefined)).toEqual({});
    expect(normaliseMacros({ '\\R': '\\mathbb{R}', bad: 'x', '\\n': 3 })).toEqual({
      '\\R': '\\mathbb{R}',
    });
    expect(currentMacros([{ macros: { '\\a': '1' } }, { macros: null }])).toEqual({});
    expect(currentMacros([{ macros: { '\\a': '1' } }, { macros: { '\\b': '2' } }])).toEqual({
      '\\b': '2',
    });
    expect(currentMacros([])).toEqual({});
  });
});

describe('mathematics in review reports', () => {
  it('renders conventional inline and display delimiters without changing mathematical content', () => {
    const text = String.raw`The bound is \(n \le 2t\).

\[
f(n)=\begin{cases}n-1 & n>0\\0 & n=0\end{cases}
\]`;
    const { container } = render(<Markdown text={text} texDelimiters lineBreaks />);
    expect(container.querySelectorAll('.katex')).toHaveLength(2);
    expect(container.querySelector('.katex-display')).not.toBeNull();
    expect(container.querySelector(KATEX_ERROR)).toBeNull();
    expect(container.querySelector('annotation')).toHaveTextContent(String.raw`n \le 2t`);
  });

  it('keeps TeX examples inside code literal and leaves existing dollar math intact', () => {
    const inline = String.raw`\(literal\)`;
    const display = String.raw`\[literal\]`;
    const text =
      '`' + inline + '`\n\n```tex\n' + display + '\n```\n\n~~~tex\n' + inline + '\n~~~\n\n$x+1$';
    const { container } = render(<Markdown text={text} texDelimiters />);
    expect(container.querySelectorAll('.katex')).toHaveLength(1);
    expect(container.querySelector('code')).toHaveTextContent(inline);
    expect(container.querySelector('pre code')).toHaveTextContent(display);
  });

  it('does not reinterpret escaped bibliography brackets in manuscript text', () => {
    const { container } = render(<LatexText source={String.raw`See \cite{Smith2026}.`} />);
    expect(container.querySelector('.katex')).toBeNull();
    expect(container).toHaveTextContent('[Smith2026]');
  });
});
