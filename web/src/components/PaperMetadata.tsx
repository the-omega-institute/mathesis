import type { Author, PublicPaper } from '../api/types';
import { canonicalAuthorName } from '../lib/authorName';
import { citationUrl } from '../lib/citation';
import { texToText } from '../lib/texText';

/** Standard citation metadata for browser reference managers; public fields only. */
export function PaperMetadata({ paper }: { paper: PublicPaper }) {
  const summary = paper.summary;
  const version = paper.versions?.at(-1);
  const origin = window.location.origin;
  return (
    <CitationMetadata
      title={summary.title}
      authors={summary.authors}
      doi={summary.doi}
      abstract={summary.abstract_text}
      canonicalUrl={citationUrl(summary, origin)}
      publicationDate={summary.accepted_at}
      pdfUrl={
        summary.submission && version?.has_pdf
          ? `${origin}/papers/${encodeURIComponent(summary.record)}/pdf`
          : undefined
      }
    />
  );
}

/** Public bibliographic facts, shared by venue records and independently archived papers. */
export function CitationMetadata({
  title,
  authors,
  doi,
  abstract,
  canonicalUrl,
  publicationDate,
  pdfUrl,
}: {
  title: string;
  authors: Author[];
  doi?: string;
  abstract?: string;
  canonicalUrl: string;
  publicationDate?: string;
  pdfUrl?: string;
}) {
  return (
    <>
      <title>{texToText(title)} · Mathesis</title>
      <link rel="canonical" href={canonicalUrl} />
      <meta name="citation_title" content={title} />
      {authors.map((author, index) => (
        <meta
          key={index}
          name="citation_author"
          content={texToText(canonicalAuthorName(author.name))}
        />
      ))}
      {publicationDate ? (
        <meta name="citation_publication_date" content={publicationDate.slice(0, 10)} />
      ) : null}
      {doi ? <meta name="citation_doi" content={doi} /> : null}
      {abstract ? <meta name="description" content={abstract} /> : null}
      {pdfUrl ? <meta name="citation_pdf_url" content={pdfUrl} /> : null}
    </>
  );
}
