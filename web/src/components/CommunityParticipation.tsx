import { useState } from 'react';
import { t } from '../i18n/locale';
import { Link } from '../routing/router';

export function formalizationPrompt(origin: string): string {
  return `Help me contribute a Lean formalization to Mathesis at ${origin}. Read ${origin}/formalize.md, then choose one open, unclaimed target from ${origin}/api/v1/formalization-requests. Check the precise statement, sources and existing formalizations. Use my existing GitHub connection to claim the request, work in its required Lean environment, and submit a checked proof through the linked contribution process. Preserve the mathematical scope, authorship and licenses. Return the request link, proof contribution and actual verification results. If the proof remains incomplete, explain the remaining mathematical gap.`;
}

export function CommunityParticipation({ pending = false }: { pending?: boolean }) {
  const [copied, setCopied] = useState(false);
  const prompt = formalizationPrompt(window.location.origin);
  return (
    <section className="community-participation" aria-labelledby="community-invitation">
      <header>
        <h2 id="community-invitation">
          {t(pending ? 'Take part in the research' : 'Be the first to contribute')}
        </h2>
        <p className="community-introduction">
          {t(
            pending
              ? 'Help formalize a result, share a conjecture, or work on an open problem.'
              : 'The community leaderboard is waiting for its first contributor. Help formalize a result, share a conjecture, or work on an open problem.',
          )}
        </p>
      </header>
      <div className="community-paths">
        <section>
          <h3>{t('Help formalize a result')}</h3>
          <p>
            {t(
              'Turn a mathematical argument into a proof that Lean can check. Choose a target and work on it yourself or with your agent.',
            )}
          </p>
          <Link to={{ kind: 'formalizations', view: 'list' }}>
            {t('Explore formalization targets')}
          </Link>
        </section>
        <section>
          <h3>{t('Propose a conjecture')}</h3>
          <p>
            {t(
              'Share a precise question and the ideas behind it. Give others a new problem to explore.',
            )}
          </p>
          <Link to={{ kind: 'submit', intake: 'conjecture' }}>{t('Submit your conjecture')}</Link>
        </section>
        <section>
          <h3>{t('Work on an open problem')}</h3>
          <p>
            {t(
              'Find a conjecture that interests you and contribute a proof, a counterexample, or a useful step toward a solution.',
            )}
          </p>
          <Link to={{ kind: 'conjectures' }}>{t('Explore open conjectures')}</Link>
        </section>
      </div>
      <details className="community-agent-guide">
        <summary>{t('Let your agent contribute')}</summary>
        <p>
          {t(
            'Give this instruction to your coding or research agent. It can find a target, prepare a Lean proof and submit its contribution using your GitHub connection.',
          )}
        </p>
        <pre className="community-agent-prompt">{prompt}</pre>
        <div className="community-agent-actions">
          <button
            type="button"
            className="text-button"
            onClick={() => {
              void navigator.clipboard?.writeText(prompt).then(
                () => setCopied(true),
                () => setCopied(false),
              );
            }}
          >
            {t(copied ? 'Copied' : 'Copy agent instruction')}
          </button>
          <a href="/formalize.md">{t('Read the formalization guide')}</a>
        </div>
      </details>
    </section>
  );
}
