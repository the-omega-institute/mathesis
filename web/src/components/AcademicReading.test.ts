import { describe, expect, it } from 'vitest';
import { academicReading } from './AcademicReading';

describe('academic paper reading', () => {
  it('retains the mathematical chapter and source links with isolated academic presentation', () => {
    const html =
      '<html lang="en"><head><link rel="stylesheet" href="tr/katex.min.css"><style>body{background:purple}</style></head><body><nav>Other project navigation</nav><main><h1>A projection bound</h1><p class="katex">x²</p><a href="witness.html">Worked example</a><img src="figure.png"><script>window.parent.bad()</script><p onclick="bad()">The proof</p></main></body></html>';
    const document = new DOMParser().parseFromString(
      academicReading(html, 'https://papers.trureturing.org/paper/en/'),
      'text/html',
    );
    expect(document.querySelector('h1')?.textContent).toBe('A projection bound');
    expect(document.querySelector('.katex')?.textContent).toBe('x²');
    expect(document.querySelector('a')?.getAttribute('href')).toBe(
      'https://papers.trureturing.org/paper/en/witness.html',
    );
    expect(document.querySelector('a')?.getAttribute('target')).toBe('_blank');
    expect(document.querySelector('base')?.href).toBe('https://papers.trureturing.org/paper/en/');
    expect(document.querySelector('link')?.href).toContain('/en/tr/katex.min.css');
    expect(document.querySelector('script')).toBeNull();
    expect(document.querySelector('[onclick]')).toBeNull();
    expect(document.querySelector('nav')).toBeNull();
    expect(document.documentElement.outerHTML).not.toContain('background:purple');
  });
});
