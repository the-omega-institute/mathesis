import { t } from '../i18n/locale';
import { Link } from '../routing/router';
import '../styles/learn-lean.css';

const firstProof = `theorem addition_commutes (a b : Nat) : a + b = b + a := by
  exact Nat.add_comm a b`;

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
              'Lean is free, open-source software for writing mathematical proofs and checking every logical step by computer.',
            )}
          </p>
          <p className="learning-next">
            <a href="#download-lean">{t('Download and install Lean')}</a>
            <a href="https://live.lean-lang.org/">{t('Try Lean in your browser')}</a>
          </p>
        </div>
      </header>
      <div className="learning-layout">
        <nav className="learning-contents" aria-label={t('Learning path')}>
          <p>{t('Learning path')}</p>
          <ul>
            <li>
              <a href="#what-is-lean">{t('What Lean does')}</a>
            </li>
            <li>
              <a href="#download-lean">{t('Where to download Lean')}</a>
            </li>
            <li>
              <a href="#check-a-proof">{t('Run and check a proof')}</a>
            </li>
            {stages.map((stage) => (
              <li key={stage.id}>
                <a href={`#${stage.id}`}>{t(stage.title)}</a>
              </li>
            ))}
            <li>
              <a href="#mathesis-contribution">{t('Your first Mathesis contribution')}</a>
            </li>
          </ul>
        </nav>
        <div className="learning-reading">
          <section aria-labelledby="what-is-lean">
            <h2 id="what-is-lean">{t('What Lean does')}</h2>
            <p>
              {t(
                'Lean is both a programming language and a proof assistant. You write down the definitions, assumptions, theorem and proof in a precise language; Lean checks that the conclusion follows from those assumptions. Writing a mathematical argument this way is called formalization.',
              )}
            </p>
            <p>
              {t(
                'Think of it as turning a written proof into a program whose logic a computer can check. A proof about all natural numbers covers every case through its reasoning, rather than testing a large sample. Constructive proofs can also describe algorithms, whose correctness Lean can verify.',
              )}
            </p>
            <p>
              {t(
                'Lean and the Mathlib mathematical library are free to use, with no subscription required. You can check proofs on your own computer. Mathlib contains reusable definitions and proofs, so a new result can build on earlier mathematics. Mathesis connects those results and records credit for the original mathematics and the formalization.',
              )}
            </p>
            <p className="learning-note">
              <a href="https://lean-lang.org/">{t('About Lean')}</a>
              {' · '}
              <a href="https://lean-lang.org/theorem_proving_in_lean4/propositions_and_proofs.html">
                {t('How proofs relate to programs')}
              </a>
            </p>
          </section>
          <section aria-labelledby="download-lean">
            <h2 id="download-lean">{t('Where to download Lean')}</h2>
            <p>
              {t(
                'Start with Lean’s official installation guide. The usual setup has two parts: VS Code, the editor in which you write proofs, and the Lean 4 extension, which helps install Lean and checks your work. Both are free.',
              )}
            </p>
            <p>
              <a href="https://lean-lang.org/install/">{t('Official Lean installation guide')}</a>
            </p>
            <ol className="learning-contribution-steps">
              <li>
                <a href="https://code.visualstudio.com/Download">{t('Download VS Code')}</a>
                <p>{t('Choose Windows, macOS or Linux, then install and open the editor.')}</p>
              </li>
              <li>
                <a href="https://marketplace.visualstudio.com/items?itemName=leanprover.lean4">
                  {t('Install the official Lean 4 extension')}
                </a>
                <p>
                  {t(
                    'Open this extension page and select Install. Allow it to open VS Code, then finish installing the extension there.',
                  )}
                </p>
              </li>
              <li>
                <strong>{t('Complete the Lean setup guide in VS Code')}</strong>
                <p>
                  {t(
                    'Follow the extension’s setup guide to install Lean. To reopen it, create an empty file, select the ∀ symbol at the top right, then Documentation → Show Setup Guide. The official installation page illustrates each step.',
                  )}
                </p>
              </li>
            </ol>
            <p>
              {t(
                'Once setup is complete, open a tutorial project or an existing Lean project. The extension helps you use the Lean version that project requires. To try a first proof immediately, the browser editor works without any download.',
              )}
            </p>
          </section>
          <section aria-labelledby="check-a-proof">
            <h2 id="check-a-proof">{t('Run and check a proof')}</h2>
            <p>
              <a href={`https://live.lean-lang.org/#code=${encodeURIComponent(firstProof)}`}>
                {t('Run this example in Lean Web')}
              </a>
              {t(
                '. The proof below opens in the editor and Lean checks it automatically. No installation is needed.',
              )}
            </p>
            <pre className="learning-code">
              <code>{firstProof}</code>
            </pre>
            <p>
              {t(
                'The first line says that a + b = b + a for any natural numbers a and b. The second line supplies the existing proof Nat.add_comm. Lean checks that this proof matches the statement. With your cursor after the proof, the information panel should show no remaining goals and no errors.',
              )}
            </p>
            <p>
              <strong>{t('On your computer.')}</strong> {t('Follow the ')}
              <a href="#download-lean">{t('download and setup steps above')}</a>
              {t(
                ' and open a .lean file inside a Lean project in VS Code. Lean checks the file as you edit; the information panel shows the current goal and any errors.',
              )}
            </p>
            <p>
              <strong>{t('Compile a project.')}</strong>{' '}
              {t(
                'Once the project and its dependencies are installed, run the following in the terminal from the project folder:',
              )}
            </p>
            <pre className="learning-code">
              <code>lake build</code>
            </pre>
            <p>
              {t(
                'Lake is Lean’s project tool. This command compiles and checks the project’s configured build targets. For an unfamiliar project, follow its README to prepare dependencies and build any additional proof files.',
              )}
            </p>
            <p className="learning-note">
              {t(
                'A finished proof also needs all gaps filled: Lean can compile a file containing sorry placeholders with a warning. Review the exact statement, assumptions and dependencies as well as the build result.',
              )}
            </p>
            <p>
              {t(
                'New to proof assistants? Start with the Natural Number Game, then work through Mathematics in Lean. If you already write Lean, go straight to Mathlib and contribution resources.',
              )}
            </p>
          </section>
          {stages.map((stage) => (
            <section key={stage.id} aria-labelledby={stage.id}>
              <h2 id={stage.id}>{t(stage.title)}</h2>
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
          <p className="learning-note">
            {t(
              'These resources are for Lean 4. Most written material is in English; the game offers a language selector.',
            )}{' '}
            <a href="https://lean-lang.org/learn/">{t('Official Lean learning directory')}</a>
          </p>
        </div>
      </div>
    </div>
  );
}
