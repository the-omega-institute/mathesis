import { useCallback, useEffect, useRef, useState } from 'react';
import { useAsync } from '../api/useAsync';
import { t } from '../i18n/locale';

/** Adapt the published chapter into Mathesis's typography. The sandbox has no
 * script, form or same-origin privileges; the original reading remains linked.
 */
export function academicReading(html: string, url: string): string {
  const source = new DOMParser().parseFromString(html, 'text/html');
  const main = source.querySelector('main');
  if (!main) throw new Error('The reading could not be displayed.');
  main
    .querySelectorAll('script,style,iframe,object,embed,form,meta,base,link')
    .forEach((node) => node.remove());
  for (const element of main.querySelectorAll('*')) {
    for (const attribute of [...element.attributes])
      if (attribute.name.startsWith('on')) element.removeAttribute(attribute.name);
  }
  for (const link of main.querySelectorAll('a')) {
    const href = link.getAttribute('href');
    if (href?.startsWith('#')) continue;
    try {
      const target = new URL(href ?? '', url);
      if (!['https:', 'http:'].includes(target.protocol)) {
        link.removeAttribute('href');
        continue;
      }
      link.href = target.href;
      link.target = '_blank';
      link.rel = 'noopener noreferrer';
    } catch {
      link.removeAttribute('href');
    }
  }
  const document = source.implementation.createHTMLDocument('');
  document.documentElement.lang = source.documentElement.lang;
  const base = document.createElement('base');
  base.href = url;
  document.head.append(base);
  for (const link of source.querySelectorAll<HTMLLinkElement>('link[rel="stylesheet"]')) {
    if (!link.getAttribute('href')?.includes('katex')) continue;
    const sheet = document.createElement('link');
    sheet.rel = 'stylesheet';
    sheet.href = new URL(link.getAttribute('href')!, url).href;
    document.head.append(sheet);
  }
  const style = document.createElement('style');
  style.textContent = `:root{color-scheme:light}*{box-sizing:border-box}body{margin:0;padding:1rem 1.25rem;color:#26352e;background:#faf9f6;font:17px/1.75 Georgia,"Noto Serif SC","Songti SC",serif}h1,h2,h3,h4{line-height:1.3;font-family:inherit;font-weight:600}h1{font-size:1.65rem;margin-top:0}h2{font-size:1.3rem;margin-top:1.7rem}h3{font-size:1.1rem}a{color:#245744;text-underline-offset:3px}h1 a,h2 a,h3 a{color:inherit;text-decoration:none}img,video{max-width:100%;height:auto}figure{margin:1.3rem 0}pre{overflow:auto;font-size:.85em}pre:has(>code[class*="typst"]){display:none}table{border-collapse:collapse;width:100%}td,th{padding:.4rem;text-align:left;vertical-align:top}.katex-display{overflow-x:auto;overflow-y:hidden}.tr-env{margin:1.5rem 0}.tr-env-head{font-weight:600}.tr-env-label,.tr-env-title{margin-right:.35em}.tr-badge{font:12px/1.4 system-ui,sans-serif;margin-left:.5em}.tr-badge-lean{color:#245744}.tr-status-written-proof .tr-badge{color:#5a45c8}.tr-lean-decl{font-size:.8em;overflow-wrap:anywhere}code{font-size:.85em}summary{cursor:pointer}`;
  document.head.append(style);
  document.body.append(document.importNode(main, true));
  return '<!doctype html>' + document.documentElement.outerHTML;
}

export function AcademicReading({ url }: { url: string }) {
  const holder = useRef<HTMLDivElement>(null);
  const [active, setActive] = useState(false);
  const load = useCallback(
    async (signal: AbortSignal) => {
      const response = await fetch(url, { signal });
      if (!response.ok) throw new Error('The reading could not be displayed.');
      const html = await response.text();
      if (html.length > 8 * 1024 * 1024) throw new Error('The reading could not be displayed.');
      return academicReading(html, url);
    },
    [url],
  );
  useEffect(() => {
    if (!holder.current || typeof IntersectionObserver === 'undefined') return;
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          setActive(true);
          observer.disconnect();
        }
      },
      { rootMargin: '200px' },
    );
    observer.observe(holder.current);
    return () => observer.disconnect();
  }, []);
  const reading = useAsync(active ? load : null);
  return (
    <div ref={holder}>
      {reading.state.status === 'ok' ? (
        <iframe
          className="material-reading-preview"
          srcDoc={reading.state.value}
          title={t('Plain-language reading')}
          sandbox="allow-popups allow-popups-to-escape-sandbox"
        />
      ) : reading.state.status === 'idle' ? (
        <button type="button" className="text-button" onClick={() => setActive(true)}>
          {t('Preview reading')}
        </button>
      ) : (
        <p className="muted">
          {t(reading.state.status === 'error' ? 'Preview unavailable' : 'Loading…')}
        </p>
      )}
    </div>
  );
}
