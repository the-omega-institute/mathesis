export type Subject = { code: string; title: string };

/** MSC 2020's two-digit classes, grouped for browsing. */
export const subjectGroups: { title: string; subjects: Subject[] }[] = [
  {
    title: 'Foundations and history',
    subjects: [
      { code: '00', title: 'General mathematics' },
      { code: '01', title: 'History and biography' },
      { code: '03', title: 'Logic and foundations' },
      { code: '97', title: 'Mathematics education' },
    ],
  },
  {
    title: 'Algebra and discrete mathematics',
    subjects: [
      { code: '05', title: 'Combinatorics' },
      { code: '06', title: 'Order and lattices' },
      { code: '08', title: 'General algebraic systems' },
      { code: '11', title: 'Number theory' },
      { code: '12', title: 'Field theory and polynomials' },
      { code: '13', title: 'Commutative algebra' },
      { code: '15', title: 'Linear algebra and matrix theory' },
      { code: '16', title: 'Associative rings and algebras' },
      { code: '17', title: 'Nonassociative rings and algebras' },
      { code: '18', title: 'Category theory and homological algebra' },
      { code: '19', title: 'K-theory' },
      { code: '20', title: 'Group theory and semigroups' },
      { code: '22', title: 'Topological groups and Lie groups' },
    ],
  },
  {
    title: 'Analysis',
    subjects: [
      { code: '26', title: 'Real functions' },
      { code: '28', title: 'Measure and integration' },
      { code: '30', title: 'Complex analysis' },
      { code: '31', title: 'Potential theory' },
      { code: '32', title: 'Several complex variables' },
      { code: '33', title: 'Special functions' },
      { code: '34', title: 'Ordinary differential equations' },
      { code: '35', title: 'Partial differential equations' },
      { code: '37', title: 'Dynamical systems and ergodic theory' },
      { code: '39', title: 'Difference and functional equations' },
      { code: '40', title: 'Sequences and series' },
      { code: '41', title: 'Approximation theory' },
      { code: '42', title: 'Harmonic analysis on Euclidean spaces' },
      { code: '43', title: 'Abstract harmonic analysis' },
      { code: '44', title: 'Integral transforms' },
      { code: '45', title: 'Integral equations' },
      { code: '46', title: 'Functional analysis' },
      { code: '47', title: 'Operator theory' },
      { code: '49', title: 'Calculus of variations and optimization' },
    ],
  },
  {
    title: 'Geometry and topology',
    subjects: [
      { code: '14', title: 'Algebraic geometry' },
      { code: '51', title: 'Geometry' },
      { code: '52', title: 'Convex and discrete geometry' },
      { code: '53', title: 'Differential geometry' },
      { code: '54', title: 'Topology' },
      { code: '55', title: 'Algebraic topology' },
      { code: '57', title: 'Manifolds and cell complexes' },
      { code: '58', title: 'Global analysis and analysis on manifolds' },
    ],
  },
  {
    title: 'Probability, statistics and computation',
    subjects: [
      { code: '60', title: 'Probability' },
      { code: '62', title: 'Statistics' },
      { code: '65', title: 'Numerical analysis' },
      { code: '68', title: 'Computer science' },
      { code: '90', title: 'Operations research' },
      { code: '91', title: 'Game theory and mathematical economics' },
      { code: '93', title: 'Systems theory and control' },
      { code: '94', title: 'Information and communication theory' },
    ],
  },
  {
    title: 'Mathematical physics and applications',
    subjects: [
      { code: '70', title: 'Mechanics of particles and systems' },
      { code: '74', title: 'Mechanics of deformable solids' },
      { code: '76', title: 'Fluid mechanics' },
      { code: '78', title: 'Optics and electromagnetic theory' },
      { code: '80', title: 'Thermodynamics and heat transfer' },
      { code: '81', title: 'Quantum theory' },
      { code: '82', title: 'Statistical mechanics' },
      { code: '83', title: 'Relativity and gravitational theory' },
      { code: '85', title: 'Astronomy and astrophysics' },
      { code: '86', title: 'Geophysics' },
      { code: '92', title: 'Mathematical biology' },
    ],
  },
];

export const subjects = subjectGroups.flatMap((group) => group.subjects);

export function findSubject(code: string): Subject | undefined {
  return subjects.find((subject) => subject.code === code);
}

const homeCodes = ['03', '05', '11', '14', '15', '20', '35', '37', '46', '54', '68', '81'];
export const homeSubjects = homeCodes.map((code) => findSubject(code)!);
