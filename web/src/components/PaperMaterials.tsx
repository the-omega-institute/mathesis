import { t, useLocale } from '../i18n/locale';
import type { PaperMaterials as Materials, PlainLanguageText } from '../api/types';
import { ExternalLink } from './ui';
import { AcademicReading } from './AcademicReading';
import { PdfPreview } from './PdfPreview';
import { hasPaperExplanation, PaperExplanation, PaperExplanationText } from './PaperExplanation';

function languageName(language: string): string {
  if (language === 'en') return t('English');
  if (language === 'zh' || language === 'zh-Hans') return t('Simplified Chinese');
  return language;
}

type PaperReadingProps = { materials?: Materials; text?: PlainLanguageText };

function hasSupplementaryMaterials(materials?: Materials): materials is Materials {
  return Boolean(materials?.slides.length || materials?.videos?.length);
}

export function PaperMaterialLinks({ materials, text }: PaperReadingProps) {
  useLocale();
  return (
    <>
      {materials?.visualizations.length ? (
        <a href="#visualizations">{t('Visualizations')}</a>
      ) : null}
      {Object.values(materials?.reading ?? {}).some(Boolean) || hasPaperExplanation(text) ? (
        <a href="#plain-language">{t('In plain language')}</a>
      ) : null}
      {hasSupplementaryMaterials(materials) ? <a href="#materials">{t('Materials')}</a> : null}
    </>
  );
}

export function PaperMaterials({ materials, text }: PaperReadingProps) {
  const locale = useLocale();
  const readings = materials?.reading ?? {};
  const reading = readings[locale] || readings.en || Object.values(readings).find(Boolean);
  return (
    <>
      {materials?.visualizations.length ? (
        <section id="visualizations" aria-labelledby="visualizations-h">
          <h2 id="visualizations-h">{t('Visualizations')}</h2>
          {materials.visualizations.map((item, index) => (
            <div className="material-visualization" key={item.id}>
              <iframe
                src={item.url}
                title={t('Interactive explanation {number}', { number: index + 1 })}
                loading="lazy"
                allow="fullscreen"
              />
              <p>
                <ExternalLink href={item.url}>{t('Open interactive explanation')}</ExternalLink>
              </p>
            </div>
          ))}
        </section>
      ) : null}
      {reading ? (
        <section id="plain-language" aria-labelledby="plain-language-h">
          <h2 id="plain-language-h">{t('In plain language')}</h2>
          <AcademicReading
            url={reading}
            fallback={hasPaperExplanation(text) ? <PaperExplanationText text={text} /> : undefined}
          />
          <p>
            <ExternalLink href={reading}>{t('Read full explanation')}</ExternalLink>
          </p>
        </section>
      ) : (
        <PaperExplanation text={text} />
      )}
      {hasSupplementaryMaterials(materials) ? (
        <SupplementaryMaterials materials={materials} />
      ) : null}
    </>
  );
}

function SupplementaryMaterials({ materials }: { materials: Materials }) {
  const locale = useLocale();
  const localizedSlides = materials.slides.filter(
    (slides) => slides.language === locale || (locale === 'zh' && slides.language === 'zh-Hans'),
  );
  const slidesToShow = localizedSlides.length
    ? localizedSlides
    : materials.slides.filter((slides) => slides.language === 'en');
  return (
    <section id="materials" aria-labelledby="materials-h">
      <h2 id="materials-h">{t('Materials')}</h2>
      {materials.slides.length ? (
        <>
          <h3>{t('Seminar slides')}</h3>
          <div className="material-slide-previews">
            {slidesToShow
              .filter((slides) => slides.pdf_url)
              .map((slides) => (
                <PdfPreview
                  key={`${slides.id}:${slides.language}`}
                  url={slides.pdf_url!}
                  title={`${t('Seminar slides')} · ${languageName(slides.language)} · ${slides.id}`}
                />
              ))}
          </div>
          <ul className="material-downloads">
            {materials.slides.map((slides) => (
              <li key={`${slides.id}:${slides.language}`}>
                {languageName(slides.language)}
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
      {materials.videos?.length ? (
        <>
          <h3>{t('Video explanations')}</h3>
          {materials.videos.map((video) => (
            <figure className="material-video" key={video.id}>
              <video
                controls
                preload="metadata"
                poster={video.poster_url ?? undefined}
                aria-label={video.title}
              >
                <source src={video.mp4_url} type="video/mp4" />
                <a href={video.mp4_url}>{t('Watch video')}</a>
              </video>
              <figcaption>
                <ExternalLink href={video.url}>{video.title}</ExternalLink>
              </figcaption>
            </figure>
          ))}
        </>
      ) : null}
    </section>
  );
}
