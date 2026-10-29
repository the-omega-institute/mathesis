import { createContext, useContext, type ReactNode } from 'react';
import ReactMarkdown, { type Components, type Options } from 'react-markdown';
import rehypeKatex from 'rehype-katex';
import remarkGfm from 'remark-gfm';
import remarkMath from 'remark-math';
import { latexToMarkdown } from '../lib/latex';
import { normaliseMacros, type Macros } from '../lib/macros';
import { reportMath } from '../lib/reportMath';

const remarkPlugins: Options['remarkPlugins'] = [remarkGfm, remarkMath];

/**
 * KaTeX plugins for one render. A malformed formula renders as red source
 * text instead of breaking the page. KaTeX writes `\gdef` definitions into
 * the macros object, so every render gets its own copy.
 */
function rehypePlugins(macros: Macros): Options['rehypePlugins'] {
  return [[rehypeKatex, { throwOnError: false, strict: 'ignore', macros: { ...macros } }]];
}

const MacrosContext = createContext<Macros>({});

/** The paper's preamble macros for every formula rendered below. Missing or null means none. */
export function MacrosProvider({ macros, children }: { macros: unknown; children: ReactNode }) {
  return (
    <MacrosContext.Provider value={normaliseMacros(macros)}>{children}</MacrosContext.Provider>
  );
}

const components: Components = {
  a: ({ href, children }) => (
    <a href={href} target="_blank" rel="noopener noreferrer">
      {children}
    </a>
  ),
};

/**
 * Markdown with GFM and TeX (`$…$`, `$$…$$`). Raw HTML in the source is not
 * rendered, and react-markdown drops `javascript:` URLs.
 */
export function Markdown({
  text,
  className,
  lineBreaks = false,
  texDelimiters = false,
}: {
  text: string;
  className?: string;
  /** Keep single line breaks, as in a letter's sign-off. */
  lineBreaks?: boolean;
  /** Reports may use conventional TeX delimiters as well as dollar math. */
  texDelimiters?: boolean;
}) {
  const macros = useContext(MacrosContext);
  const content = texDelimiters ? reportMath(text) : text;
  return (
    <div className={className ? `prose ${className}` : 'prose'}>
      <ReactMarkdown
        remarkPlugins={remarkPlugins}
        rehypePlugins={rehypePlugins(macros)}
        components={components}
      >
        {lineBreaks ? hardBreaks(content) : content}
      </ReactMarkdown>
    </div>
  );
}

/**
 * Turns single newlines into Markdown hard breaks, outside display math
 * (`$$…$$`) and fenced code, so a sign-off keeps its lines.
 */
export function hardBreaks(text: string): string {
  return text
    .split(/(\$\$[\s\S]*?\$\$|```[\s\S]*?```)/)
    .map((part, i) => (i % 2 === 1 ? part : part.replace(/([^\n])\n(?=[^\n])/g, '$1  \n')))
    .join('');
}

/** A statement written in LaTeX (as read from the author's source), rendered with KaTeX. */
export function LatexText({ source, className }: { source: string; className?: string }) {
  return <Markdown text={latexToMarkdown(source)} className={className} />;
}

/** Inline paper titles: math is rendered without introducing nested links or block markup. */
export function InlineLatex({ source }: { source: string }) {
  const macros = useContext(MacrosContext);
  const span = ({ children }: { children?: ReactNode }) => <span>{children}</span>;
  return (
    <span className="inline-latex">
      <ReactMarkdown
        remarkPlugins={[remarkMath]}
        rehypePlugins={rehypePlugins(macros)}
        skipHtml
        components={{
          p: span,
          a: span,
          h1: span,
          h2: span,
          h3: span,
          h4: span,
          h5: span,
          h6: span,
          ul: span,
          ol: span,
          li: span,
          blockquote: span,
          pre: span,
        }}
        disallowedElements={['img', 'hr']}
        unwrapDisallowed
      >
        {latexToMarkdown(source).replace(/\n/g, ' ')}
      </ReactMarkdown>
    </span>
  );
}
