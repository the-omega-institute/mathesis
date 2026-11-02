/** A guide to the research cycle, not live execution progress. */
export function ResearchCycle() {
  return (
    <figure className="home-cycle">
      <figcaption>Research, led by people</figcaption>
      <svg
        viewBox="0 0 640 410"
        role="img"
        aria-labelledby="research-cycle-title"
        aria-describedby="research-cycle-desc"
      >
        <title id="research-cycle-title">People build a shared mathematical knowledge base</title>
        <desc id="research-cycle-desc">
          People submit papers and conjectures. AI review compares them with existing knowledge,
          assesses new mathematical information and gives a decision and next steps. Accepted
          mathematics credits its authors and sources. People contribute proofs, intermediate
          results and formalizations with AI assistance. Lean declarations and checked proof
          dependencies form the core of a growing knowledge graph, supporting further review and new
          questions. Written results and citations retain their own evidence and attribution.
        </desc>
        <defs>
          <marker
            id="research-cycle-arrow"
            viewBox="0 0 8 8"
            refX="6"
            refY="4"
            markerWidth="8"
            markerHeight="8"
            orient="auto"
          >
            <path d="M1 1 L6 4 L1 7" />
          </marker>
        </defs>
        <g className="home-cycle-paths" markerEnd="url(#research-cycle-arrow)">
          <path d="M418 65 C482 76 530 106 542 140" />
          <path d="M544 236 C524 291 478 330 420 343" />
          <path d="M220 343 C156 330 105 291 91 236" />
          <path d="M92 140 C109 100 158 72 220 61" />
        </g>
        <g className="home-cycle-labels" textAnchor="middle">
          <text x="320" y="34">
            People ask &amp; build
          </text>
          <text x="320" y="59" className="home-cycle-detail">
            Papers, notes &amp; conjectures
          </text>
          <text x="530" y="173">
            Review &amp; compare
          </text>
          <text x="530" y="198" className="home-cycle-detail">
            New mathematical information
          </text>
          <text x="530" y="218" className="home-cycle-detail">
            Decision &amp; next steps
          </text>
          <text x="320" y="360">
            Accepted mathematics
          </text>
          <text x="320" y="385" className="home-cycle-detail">
            Results, sources &amp; credit
          </text>
          <text x="110" y="173">
            Prove &amp; formalize
          </text>
          <text x="110" y="198" className="home-cycle-detail">
            People with AI assistance
          </text>
          <text x="110" y="218" className="home-cycle-detail">
            Reusable Lean results
          </text>
        </g>
        <g className="home-cycle-center" textAnchor="middle">
          <text x="320" y="176">
            <tspan x="320">Lean knowledge</tspan>
            <tspan x="320" dy="28">
              graph
            </tspan>
          </text>
          <text x="320" y="232" className="home-cycle-detail">
            A foundation to build on
          </text>
        </g>
      </svg>
    </figure>
  );
}
