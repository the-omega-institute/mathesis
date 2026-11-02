import { useId, useState } from 'react';
import type { PlainLanguageText } from '../api/types';

export function PaperExplanation({ text }: { text?: PlainLanguageText | null }) {
  const [language, setLanguage] = useState<'en' | 'zh'>('en');
  const heading = useId();
  if (!text?.en.trim() || !text.zh.trim()) return null;
  return (
    <section className="paper-explanation" aria-labelledby={heading}>
      <div className="paper-explanation-heading">
        <h2 id={heading}>{language === 'en' ? 'In plain language' : '白话解读'}</h2>
        <div className="paper-explanation-languages" role="group" aria-label="Explanation language">
          <button type="button" aria-pressed={language === 'en'} onClick={() => setLanguage('en')}>
            English
          </button>
          <button
            type="button"
            lang="zh-Hans"
            aria-pressed={language === 'zh'}
            onClick={() => setLanguage('zh')}
          >
            中文
          </button>
        </div>
      </div>
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
