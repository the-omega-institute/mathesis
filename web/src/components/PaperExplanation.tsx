import { useId } from 'react';
import { t, useLocale } from '../i18n/locale';
import type { PlainLanguageText } from '../api/types';

export function PaperExplanation({ text }: { text?: PlainLanguageText | null }) {
  const language = useLocale();
  const heading = useId();
  if (!text?.en.trim() || !text.zh.trim()) return null;
  return (
    <section className="paper-explanation" aria-labelledby={heading}>
      <h2 id={heading}>{t('In plain language')}</h2>
      <div lang={language === 'en' ? 'en' : 'zh-Hans'}>
        {text[language]
          .split(/\n\s*\n/)
          .filter((paragraph) => paragraph.trim())
          .map((paragraph, index) => (
            <p key={index}>{paragraph}</p>
          ))}
      </div>
    </section>
  );
}
