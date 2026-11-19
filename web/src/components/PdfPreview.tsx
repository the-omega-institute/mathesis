import { useEffect, useRef, useState } from 'react';
import type { PDFDocumentProxy } from 'pdfjs-dist';
import { t } from '../i18n/locale';

/** Render the existing PDF with the bundled reader, including mobile browsers. */
export function PdfPreview({ url, title }: { url: string; title: string }) {
  return <PdfDocument key={url} url={url} title={title} />;
}

function PdfDocument({ url, title }: { url: string; title: string }) {
  const holder = useRef<HTMLDivElement>(null),
    canvas = useRef<HTMLCanvasElement>(null);
  const [active, setActive] = useState(false),
    [pdf, setPdf] = useState<PDFDocumentProxy>(),
    [page, setPage] = useState(1),
    [width, setWidth] = useState(800),
    [error, setError] = useState(false);
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
  useEffect(() => {
    if (!holder.current || typeof ResizeObserver === 'undefined') return;
    const observer = new ResizeObserver((entries) => {
      if (entries[0]?.contentRect.width) setWidth(entries[0].contentRect.width);
    });
    observer.observe(holder.current);
    return () => observer.disconnect();
  }, []);
  useEffect(() => {
    if (!active) return;
    let stopped = false,
      task: ReturnType<(typeof import('pdfjs-dist'))['getDocument']> | undefined;
    void (async () => {
      const [reader, worker] = await Promise.all([
        import('pdfjs-dist'),
        import('pdfjs-dist/build/pdf.worker.min.mjs?url'),
      ]);
      if (stopped) return;
      reader.GlobalWorkerOptions.workerSrc = worker.default;
      task = reader.getDocument({ url, useSystemFonts: true });
      const document = await task.promise;
      if (!stopped) setPdf(document);
    })().catch(() => {
      if (!stopped) setError(true);
    });
    return () => {
      stopped = true;
      void task?.destroy();
    };
  }, [active, url]);
  useEffect(() => {
    const element = canvas.current;
    if (!pdf || !element) return;
    let stopped = false,
      render: ReturnType<Awaited<ReturnType<PDFDocumentProxy['getPage']>>['render']> | undefined;
    void (async () => {
      const sheet = await pdf.getPage(page);
      if (stopped) return;
      const viewport = sheet.getViewport({
        scale:
          (Math.max(1, width) * (window.devicePixelRatio || 1)) /
          sheet.getViewport({ scale: 1 }).width,
      });
      element.width = viewport.width;
      element.height = viewport.height;
      const context = element.getContext('2d');
      if (!context) throw new Error('Canvas unavailable');
      render = sheet.render({ canvas: element, canvasContext: context, viewport });
      await render.promise;
    })().catch(() => {
      if (!stopped) setError(true);
    });
    return () => {
      stopped = true;
      render?.cancel();
    };
  }, [pdf, page, width]);
  return (
    <div ref={holder} className="material-pdf-preview" role="group" aria-label={title}>
      {error ? (
        <p>
          {t('Preview unavailable')} · <a href={url}>{t('Open PDF')}</a>
        </p>
      ) : (
        <>
          <canvas ref={canvas} role="img" aria-label={title} hidden={!pdf} />
          {pdf ? (
            <nav aria-label={t('Slide navigation')}>
              <button
                type="button"
                className="text-button"
                disabled={page <= 1}
                onClick={() => setPage((value) => value - 1)}
              >
                {t('Previous slide')}
              </button>
              <span>
                {page} / {pdf.numPages}
              </span>
              <button
                type="button"
                className="text-button"
                disabled={page >= pdf.numPages}
                onClick={() => setPage((value) => value + 1)}
              >
                {t('Next slide')}
              </button>
            </nav>
          ) : (
            <button className="text-button" type="button" onClick={() => setActive(true)}>
              {t(active ? 'Loading…' : 'Preview slides')}
            </button>
          )}
        </>
      )}
    </div>
  );
}
