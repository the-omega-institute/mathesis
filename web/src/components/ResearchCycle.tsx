/** A reading guide to the research cycle, rather than live execution progress. */
export function ResearchCycle() {
  return (
    <figure className="home-cycle">
      <figcaption>Research, led by people</figcaption>
      <svg
        viewBox="0 0 390 360"
        role="img"
        aria-labelledby="research-cycle-title"
        aria-describedby="research-cycle-desc"
      >
        <title id="research-cycle-title">People build a shared mathematical knowledge base</title>
        <desc id="research-cycle-desc">
          Authors and mathematicians pose questions and submit manuscripts. AI contributes review
          and computation, producing a decision and a report. Accepted results are published with
          credit. The community builds on these results, extends them and contributes formal proofs,
          leading to new questions and manuscripts.
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
          <path d="M260 56 C307 70 335 99 335 126" />
          <path d="M335 207 C326 249 294 277 264 290" />
          <path d="M135 290 C91 276 57 244 57 207" />
          <path d="M57 126 C57 91 91 63 132 52" />
        </g>
        <g className="home-cycle-labels" textAnchor="middle">
          <text x="195" y="30">
            Authors &amp; mathematicians
          </text>
          <text x="195" y="51" className="home-cycle-detail">
            Questions &amp; manuscripts
          </text>
          <text x="309" y="153">
            AI review
          </text>
          <text x="309" y="174" className="home-cycle-detail">
            Check &amp; compute
          </text>
          <text x="309" y="193" className="home-cycle-detail">
            Decision &amp; report
          </text>
          <text x="195" y="311">
            Accepted results
          </text>
          <text x="195" y="332" className="home-cycle-detail">
            Publication &amp; credit
          </text>
          <text x="79" y="153">
            Community
          </text>
          <text x="79" y="174" className="home-cycle-detail">
            Extend &amp; formalize
          </text>
          <text x="79" y="193" className="home-cycle-detail">
            Build on earlier work
          </text>
        </g>
        <text x="195" y="157" textAnchor="middle" className="home-cycle-center">
          <tspan x="195">A growing</tspan>
          <tspan x="195" dy="24">
            knowledge base
          </tspan>
        </text>
      </svg>
    </figure>
  );
}
