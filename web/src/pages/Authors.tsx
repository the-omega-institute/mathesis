import { Link } from '../routing/router';

export function AuthorsPage() {
  return (
    <div className="page narrow-page">
      <header className="page-head">
        <div>
          <p className="eyebrow">Research archive</p>
          <h1>Authors</h1>
          <p className="lede">
            Author records are part of the research archive. Search there to find a researcher and
            see their papers, conjectures and formalization work.
          </p>
        </div>
      </header>
      <p>
        <Link className="button" to={{ kind: 'papers' }}>
          Search the research archive
        </Link>
      </p>
    </div>
  );
}
