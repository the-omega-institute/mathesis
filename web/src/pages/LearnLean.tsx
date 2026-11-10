import { t } from '../i18n/locale';
import { Link } from '../routing/router';
import '../styles/learn-lean.css';

const stages = [
  {
    id: 'first-proof',
    title: 'Try your first proof',
    introduction:
      'Begin in your browser. Learn to read the current goal, use a hypothesis, and check each step of a short proof.',
    resources: [
      {
        title: 'Natural Number Game',
        url: 'https://adam.math.hhu.de/#/g/leanprover-community/NNG4',
        detail: 'Interactive game · Lean community · Multiple languages',
        description:
          'A gentle introduction through arithmetic puzzles. Practice rewriting, induction and basic proof tactics with immediate feedback. Start here if you have never used a proof assistant.',
      },
      {
        title: 'Lean Web',
        url: 'https://live.lean-lang.org/',
        detail: 'Browser editor · Lean · English interface',
        description:
          'Run small Lean examples without a local installation. Change a hypothesis or proof step and watch how the goal changes; use it alongside the books below.',
      },
    ],
  },
  {
    id: 'mathematical-proofs',
    title: 'Write mathematical proofs in Lean',
    introduction:
      'Move from guided puzzles to ordinary mathematics. Work through exercises as you read; a small proof that you can explain is a useful first milestone.',
    resources: [
      {
        title: 'Mathematics in Lean',
        url: 'https://leanprover-community.github.io/mathematics_in_lean/',
        detail: 'Textbook with exercises · Lean community · English',
        description:
          'The main starting point for mathematicians. Learn tactic proofs using familiar examples from logic, sets, functions, algebra and analysis, with exercises in a companion Lean project.',
      },
      {
        title: 'Theorem Proving in Lean 4',
        url: 'https://lean-lang.org/theorem_proving_in_lean4/',
        detail: 'Foundations textbook · Lean · English',
        description:
          'Understand the language behind the tactics: propositions, dependent types, quantifiers, equality and inductive definitions. Read the relevant chapters when you want to understand why a proof works.',
      },
      {
        title: 'Install Lean',
        url: 'https://lean-lang.org/install/',
        detail: 'Installation guide · Lean · English',
        description:
          'Set up Lean and its VS Code extension when you are ready to work on larger examples. Open a tutorial or an existing project and use the Lean version that project specifies.',
      },
    ],
  },
  {
    id: 'mathlib',
    title: 'Build on Mathlib',
    introduction:
      'Most research formalizations reuse existing definitions and lemmas. Learn to find a suitable theorem, read its assumptions, and adapt it to your problem.',
    resources: [
      {
        title: 'Mathlib documentation',
        url: 'https://leanprover-community.github.io/mathlib4_docs/',
        detail: 'Mathematical library reference · Lean community · English',
        description:
          'Browse definitions, theorem statements and source files across the library. Follow the source of a nearby result to see its imports and how its proof uses earlier mathematics.',
      },
      {
        title: 'Loogle',
        url: 'https://loogle.lean-lang.org/',
        detail: 'Theorem search · Lean community · English interface',
        description:
          'Search by the shape of a theorem or part of its type when you do not know its name. Check the returned statement and hypotheses before using it in your proof.',
      },
      {
        title: 'Working with Lean projects',
        url: 'https://leanprover-community.github.io/install/project.html',
        detail: 'Project guide · Lean community · English',
        description:
          'Learn how to open an existing project, obtain its Mathlib dependencies and keep a working environment. Use the project’s pinned versions so examples and imports agree.',
      },
    ],
  },
  {
    id: 'community',
    title: 'Ask questions and contribute',
    introduction:
      'You can start with a supporting lemma, a clearer statement or a small extension. Share a minimal example when asking for help so others can reproduce the question.',
    resources: [
      {
        title: 'Lean community on Zulip',
        url: 'https://leanprover.zulipchat.com/',
        detail: 'Questions and discussion · Lean community · Mainly English',
        description:
          'Ask beginner questions, discuss formalizations and find people working in your area. Read the community guidance and include your Lean code, error message and project version when you need help.',
      },
      {
        title: 'Contributing to Mathlib',
        url: 'https://leanprover-community.github.io/contribute/index.html',
        detail: 'Contribution guide · Lean community · English',
        description:
          'Learn the library’s conventions and review process when a result could be useful beyond one project. This is the guide for contributing to Mathlib itself.',
      },
    ],
  },
];

export function LearnLeanPage() {
  return (
    <div className="page learn-lean-page">
      <header className="page-head">
        <div>
          <p className="learning-parent">
            <Link to={{ kind: 'formalizations' }}>{t('Formalizations')}</Link>
          </p>
          <h1>{t('Learn Lean')}</h1>
          <p className="lede">
            {t(
              'A path from your first checked proof to contributing mathematical formalizations, with selected tutorials, tools and community resources.',
            )}
          </p>
          <p className="learning-start">
            {t(
              'New to proof assistants? Start with the Natural Number Game, then work through Mathematics in Lean. If you already write Lean, go straight to Mathlib and contribution resources.',
            )}
          </p>
        </div>
      </header>
      <div className="learning-layout">
        <nav className="learning-contents" aria-label={t('Learning path')}>
          <p>{t('Learning path')}</p>
          <ol>
            {stages.map((stage) => (
              <li key={stage.id}>
                <a href={`#${stage.id}`}>{t(stage.title)}</a>
              </li>
            ))}
            <li>
              <a href="#mathesis-contribution">{t('Your first Mathesis contribution')}</a>
            </li>
          </ol>
          <p className="learning-note">
            {t(
              'All resources below are for Lean 4. Most written material is in English; the game offers a language selector.',
            )}
          </p>
          <a href="https://lean-lang.org/learn/">{t('Official Lean learning directory')}</a>
        </nav>
        <div className="learning-reading">
          {stages.map((stage, index) => (
            <section key={stage.id} aria-labelledby={stage.id}>
              <h2 id={stage.id}>
                <span className="learning-number" aria-hidden="true">
                  {index + 1}.
                </span>{' '}
                {t(stage.title)}
              </h2>
              <p className="learning-introduction">{t(stage.introduction)}</p>
              <ul className="learning-resources">
                {stage.resources.map((resource) => (
                  <li key={resource.url}>
                    <h3>
                      <a href={resource.url}>{t(resource.title)}</a>
                    </h3>
                    <p className="learning-resource-detail">{t(resource.detail)}</p>
                    <p>{t(resource.description)}</p>
                  </li>
                ))}
              </ul>
            </section>
          ))}
          <section aria-labelledby="mathesis-contribution">
            <h2 id="mathesis-contribution">{t('Your first Mathesis contribution')}</h2>
            <p>
              {t(
                'Choose a result you understand. Read its mathematical statement and any existing Lean source, then look for a small, precise piece of proof work you can add.',
              )}
            </p>
            <ol className="learning-contribution-steps">
              <li>
                {t(
                  'Explore the dependency graph and distinguish existing formalizations from open proof work.',
                )}
              </li>
              <li>
                {t(
                  'For a confirmed target, keep its assumptions and prove the exact statement. You can also share an existing Lean project or a permanent source link with its original attribution.',
                )}
              </li>
              <li>
                {t(
                  'Submit your contribution with the corresponding result. Independently verified proof contributions receive credit; the original mathematical authors remain credited separately.',
                )}
              </li>
            </ol>
            <p className="learning-next">
              <Link to={{ kind: 'formalizations' }}>{t('Explore formalizations')}</Link>
              <Link to={{ kind: 'submit', intake: 'lean' }}>{t('Contribute Lean work')}</Link>
              <Link to={{ kind: 'leaderboard', category: 'formalization' }}>
                {t('Leaderboard')}
              </Link>
            </p>
          </section>
        </div>
      </div>
    </div>
  );
}
