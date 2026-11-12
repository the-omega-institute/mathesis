import { t, useLocale } from '../i18n/locale';
import type { PaperMaterials as Materials } from '../api/types';
import { ExternalLink } from './ui';

function languageName(language: string): string {
  if (language === 'en') return t('English');
  if (language === 'zh' || language === 'zh-Hans') return t('Simplified Chinese');
  return language;
}

export function PaperMaterials({ materials }: { materials?: Materials }) {
  useLocale();
  if (!materials) return null;
  return (
    <section id="materials" aria-labelledby="materials-h">
      <h2 id="materials-h">{t('Materials')}</h2>
      <p>
        {t(
          'Prepared by Mathesis from the manuscript; explanations and illustrations are not verification.',
        )}
      </p>
      {Object.keys(materials.reading).length ? (
        <>
          <h3>{t('Plain-language reading')}</h3>
          <ul>
            {Object.entries(materials.reading).map(([language, url]) => (
              <li key={language}>
                <ExternalLink href={url}>{languageName(language)}</ExternalLink>
              </li>
            ))}
          </ul>
        </>
      ) : null}
      {materials.slides.length ? (
        <>
          <h3>{t('Seminar slides')}</h3>
          <ul>
            {materials.slides.map((slides) => (
              <li key={`${slides.id}:${slides.language}`}>
                {languageName(slides.language)} · {slides.id}
                {slides.pdf_url ? (
                  <>
                    {' '}
                    · <ExternalLink href={slides.pdf_url}>{t('PDF')}</ExternalLink>
                  </>
                ) : null}
                {slides.notes_url ? (
                  <>
                    {' '}
                    · <ExternalLink href={slides.notes_url}>{t('Notes')}</ExternalLink>
                  </>
                ) : null}
              </li>
            ))}
          </ul>
        </>
      ) : null}
      {materials.visualizations.length ? (
        <>
          <h3>{t('Interactive visualizations')}</h3>
          <ul>
            {materials.visualizations.map((item) => (
              <li key={item.id}>
                <ExternalLink href={item.url}>{item.id}</ExternalLink>
              </li>
            ))}
          </ul>
        </>
      ) : null}
      <p>
        <ExternalLink href={materials.repository_url}>{t('Repository')}</ExternalLink>
      </p>
    </section>
  );
}
