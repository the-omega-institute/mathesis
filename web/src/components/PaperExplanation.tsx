import { useId } from 'react';
import { t, useLocale } from '../i18n/locale';
import type { PlainLanguageText } from '../api/types';

export function hasPaperExplanation(text?: PlainLanguageText | null): text is PlainLanguageText {
  return Boolean(text?.en.trim() && text.zh.trim());
}

export function PaperExplanationText({ text }: { text: PlainLanguageText }) {
  const language = useLocale();
  return (
    <div lang={language === 'en' ? 'en' : 'zh-Hans'}>
      {text[language]
        .split(/\n\s*\n/)
        .filter((paragraph) => paragraph.trim())
        .map((paragraph, index) => (
          <p key={index}>{paragraph}</p>
        ))}
    </div>
  );
}

export function PaperExplanation({ text }: { text?: PlainLanguageText | null }) {
  const language = useLocale();
  const heading = useId();
  if (!hasPaperExplanation(text)) return null;
  return (
    <section id="plain-language" className="paper-explanation" aria-labelledby={heading}>
      <h2 id={heading} lang={language === 'zh' ? 'zh-Hans' : 'en'}>
        {t('In plain language')}
      </h2>
      <PaperExplanationText text={text} />
    </section>
  );
}
