import './paper-research.css';
import { useState } from 'react';
import { t } from '../i18n/locale';

export function submissionPrompt(origin: string): string {
  return `Submit the paper in this folder to Mathesis at ${origin}. Read ${origin}/skill.md first. Prepare the manuscript, inspect the compiled PDF, and check the extracted title, abstract and authors against the source. Use my existing NyxID connection. Follow my publication preference, preserve author attribution, and submit for review once the source and metadata are complete. Return the private submission link and its current status. Do not upload agent sessions or unrelated files.`;
}

export function AgentSubmission({ compact = false }: { compact?: boolean }) {
  const [copied, setCopied] = useState(false);
  const prompt = submissionPrompt(window.location.origin);
  const contents = (
    <>
      <p>
        {t(
          'Ask your research agent to prepare and submit your manuscript using the same submission process.',
        )}
      </p>
      <p>
        <a href="/skill.md">{t('Read the agent submission guide')}</a>
      </p>
      <pre className="agent-submission-prompt">{prompt}</pre>
      <button
        className="text-button"
        type="button"
        onClick={() => {
          void navigator.clipboard?.writeText(prompt).then(
            () => setCopied(true),
            () => setCopied(false),
          );
        }}
      >
        {t(copied ? 'Copied' : 'Copy submission instruction')}
      </button>
    </>
  );
  return compact ? (
    <details className="agent-submission history">
      <summary>{t('Submit with your agent')}</summary>
      {contents}
    </details>
  ) : (
    <section className="agent-submission">
      <h2>{t('Submit with your agent')}</h2>
      {contents}
    </section>
  );
}
