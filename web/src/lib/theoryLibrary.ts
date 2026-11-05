import areaData from '../data/theory-areas.json';
import catalog from '../data/mathlib-catalog.json';
import type { Locale } from '../i18n/locale';

export type LocalizedText = { en: string; zh: string };
export type TheoryArea = (typeof areaData)[number];
export const theoryAreas = areaData;
export const formalLibrary = catalog;
export const libraryGroups = [
  { title: 'Foundations and language', areas: ['foundations', 'data', 'order', 'categories'] },
  {
    title: 'Algebra and discrete structures',
    areas: ['algebra', 'linear-algebra', 'number-theory', 'combinatorics'],
  },
  { title: 'Spaces, limits and change', areas: ['analysis', 'topology', 'geometry', 'dynamics'] },
  { title: 'Probability and formal methods', areas: ['probability', 'formal-methods'] },
];
export const localize = (text: LocalizedText, locale: Locale) => text[locale];
export const findTheoryArea = (id: string) => theoryAreas.find((area) => area.id === id);
export const areaModules = (area: TheoryArea) =>
  catalog.modules.filter((name) => area.roots.includes(name.split('.')[1]!));
export const moduleSource = (name: string) =>
  `${catalog.repository}/blob/${catalog.revision}/${name.replaceAll('.', '/')}.lean`;
export const directorySource = (name: string) =>
  `${catalog.repository}/tree/${catalog.revision}/${name.replaceAll('.', '/')}`;
export function moduleChildren(prefix: string, modules: readonly string[]) {
  return [
    ...new Set(
      modules
        .filter((name) => name.startsWith(prefix + '.'))
        .map((name) => name.slice(prefix.length + 1).split('.')[0]!),
    ),
  ].sort();
}
/** Readable navigation labels, never theorem titles or inferred mathematical statements. */
export function moduleLabel(name: string) {
  return name.replace(/([a-z\d])([A-Z])/g, '$1 $2').replace(/([A-Z])([A-Z][a-z])/g, '$1 $2');
}

/** Editorial topic placement from the published manuscript; not new MSC metadata or proof use. */
const collectedAreas: Readonly<Record<string, readonly string[]>> = {
  'chen-chen-step-m-eulerian-parity-positivity': ['combinatorics', 'analysis'],
  'oeis-a189733-curtz-denominator-conjecture': ['number-theory', 'combinatorics'],
  'kmvq31-sl3-obstruction': ['algebra', 'linear-algebra'],
  'oeis-a381423-hermite-differences': ['analysis', 'combinatorics'],
  'mathoverflow-469781-weighted-orthogonal-minors': ['linear-algebra'],
  'chiral-event-parity-minimality': ['topology', 'geometry'],
  'huang-stoica-xia-zhong-2020-tree-equations-nonconstant': ['geometry', 'combinatorics'],
  'bajnok-beatty-original-conjecture-4-1': ['algebra', 'combinatorics'],
  'okada-2020-premerged-determinant-nonpalindromic': ['linear-algebra', 'combinatorics'],
  'stepanov-herrmann-guhr-2021-correlation-top-eigenvector-weight': [
    'linear-algebra',
    'probability',
  ],
  'koljancic-oblak-2026-question-5-11-even-loop-path': ['linear-algebra', 'combinatorics'],
  'peled-teeni-carmi-cohen-2021-correlation-minor-norm-fnf': ['probability', 'linear-algebra'],
  ggps2025: ['analysis'],
  'mathoverflow-513954-singular-diagonal-downdate-inertia': ['linear-algebra', 'topology'],
  'malkoun-2026-mo512579-jordan-elementary': ['linear-algebra'],
  'kamin-shchukin-schmidt-van-loock-2023-swap-earliest-dephasing': ['probability', 'combinatorics'],
  'consiglio-apollaro-wiesniak-2022-xmems-css-shape': ['probability', 'linear-algebra'],
  'koljancic-oblak-2026-nssp-minimum-arcs-problem-1-8': ['linear-algebra', 'combinatorics'],
  'hexagon-perp-line-counterexamples': ['geometry', 'combinatorics', 'probability'],
  'pozsgay-wanless-nonlinear-classical-factors': ['probability', 'linear-algebra', 'combinatorics'],
  'alikhani-griswold-2026-connected-minus-four-family': ['combinatorics'],
  'xu-et-al-2023-noisy-ghz-witness-bound-gap': ['probability', 'linear-algebra'],
  'radsarar-anando-2026-apollonian-second-eigenvalue': ['linear-algebra', 'combinatorics'],
  'tarabunga-frau-haug-tirrito-piroli-2025-shannon-branch-bound': ['probability'],
  'oeis-a378495-difference-conjectures': ['combinatorics', 'number-theory'],
  'hardin-barker-a209723-edge-differences': ['combinatorics'],
  'oeis-a391810-alternating-jacobsthal': ['number-theory', 'combinatorics'],
  'huang-kosugi-nishi-matsushita-2024-reflection-diagonal-cnot-lower-bound': [
    'probability',
    'formal-methods',
  ],
  'oeis-a396659-edge-collision-ratio': ['combinatorics'],
  'b164660-chebfloor': ['analysis', 'number-theory'],
  'bu-gu-jaffe-2025-mixed-state-inverse-sumset': ['probability'],
  'user3-2026-orthogonal-absolute-product-minimum': ['linear-algebra'],
  'kang-jager-2026-universal-permutative-lift-refutation': ['dynamics', 'topology'],
  'wang-conjecture326': ['algebra', 'combinatorics'],
  'denis-lacaille-martin-serrano-ensastiga-2026-anticoherence-trace-distance': [
    'probability',
    'linear-algebra',
  ],
  'mathar-a027182-closed-form': ['combinatorics', 'number-theory'],
  kurk396563: ['combinatorics', 'linear-algebra'],
  bala295462: ['number-theory', 'combinatorics'],
};
export function researchAreas(work: string, msc: readonly string[] = []) {
  // Supplied subject classifications take precedence over local editorial indexing.
  if (msc.length)
    return theoryAreas
      .filter((area) => msc.some((code) => area.msc.includes(code.slice(0, 2))))
      .map((area) => area.id);
  return collectedAreas[work] ?? [];
}
