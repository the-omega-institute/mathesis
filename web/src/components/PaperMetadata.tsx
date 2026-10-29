import type { PublicPaper } from '../api/types';
import { pdfHref } from '../api/client';
import { citationUrl } from '../lib/citation';
import { texToText } from '../lib/texText';

/** Standard citation metadata for browser reference managers; public fields only. */
export function PaperMetadata({ paper }: { paper: PublicPaper }) {
  const summary = paper.summary;
  const version = paper.versions?.at(-1);
  const origin = window.location.origin;
  return (
    <>
      <title>{texToText(summary.title)} · Mathesis</title>
      <link rel="canonical" href={citationUrl(summary, origin)} />
      <meta name="citation_title" content={summary.title} />
      {summary.authors.map((author, index) => (
        <meta key={index} name="citation_author" content={texToText(author.name)} />
      ))}
      {summary.accepted_at ? (
        <meta name="citation_publication_date" content={summary.accepted_at.slice(0, 10)} />
      ) : null}
      {summary.doi ? <meta name="citation_doi" content={summary.doi} /> : null}
      {summary.abstract_text ? <meta name="description" content={summary.abstract_text} /> : null}
      {summary.submission && version?.has_pdf ? (
        <meta
          name="citation_pdf_url"
          content={origin + pdfHref(summary.submission, version.number)}
        />
      ) : null}
    </>
  );
}
