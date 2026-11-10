# Google Scholar crawlability

Mathesis now exposes a crawlable representation for every public accepted
record. The interactive React page remains at `/papers/{record}`; the crawler
representation is `/papers/{record}/landing.html`, and the latest public PDF is
at `/papers/{record}/pdf`.

The HTML landing response contains the title, visible author list, abstract,
publication date, canonical URL, and Scholar-compatible metadata:

- `citation_title`
- one `citation_author` tag per author
- `citation_publication_date`
- `citation_pdf_url` when a compiled PDF exists
- `citation_fulltext_html_url`

The public web proxy serves `/sitemap.xml` from the accepted-record store and
serves `/robots.txt` with the sitemap URL for the current host. The sitemap
includes only records whose analysis visibility is public. Private accepted
records therefore cannot appear in either the crawlable HTML or the sitemap.

The API also updates the interactive page's document head after its public
record loads. This keeps normal browser navigation and crawlers that execute
JavaScript consistent with the direct HTML response.

Before asking Google Scholar to recrawl a deployment, verify the public origin
anonymously:

```sh
curl -fsSL https://<public-host>/robots.txt
curl -fsSL https://<public-host>/sitemap.xml
curl -fsSL https://<public-host>/papers/WP-2026-0001/landing.html
curl -fsSI https://<public-host>/papers/WP-2026-0001/pdf
```

The final step is operational: deploy the web and API images containing these
routes, then wait for Scholar's normal crawl cycle. The implementation makes
the pages eligible for discovery and parsing; Google Scholar still controls
when a record appears in search results.
