import { t, useLocale } from '../i18n/locale';
import type { PaperMaterials as Materials } from '../api/types';
import { ExternalLink } from './ui';

function languageName(language: string): string {
  if (language === 'en') return t('English');
  if (language === 'zh' || language === 'zh-Hans') return t('Simplified Chinese');
  return language;
}

export function PaperMaterials({ materials }: { materials?: Materials }) {
  const locale = useLocale();
  if (!materials) return null;
  const reading =
    materials.reading[locale] ?? materials.reading.en ?? Object.values(materials.reading)[0];
  const localizedSlides = materials.slides.filter(
    (slides) => slides.language === locale || (locale === 'zh' && slides.language === 'zh-Hans'),
  );
  const slidesToShow = localizedSlides.length
    ? localizedSlides
    : materials.slides.filter((slides) => slides.language === 'en');
  return (
    <section id="materials" aria-labelledby="materials-h">
      <h2 id="materials-h">{t('Materials')}</h2>
      {Object.keys(materials.reading).length ? (
        <>
          <h3>{t('Plain-language reading')}</h3>
          {reading ? (
            <iframe
              className="material-reading-preview"
              src={reading}
              title={t('Plain-language reading')}
              loading="lazy"
            />
          ) : null}
          <ul className="material-downloads">
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
          <div className="material-slide-previews">
            {slidesToShow
              .filter((slides) => slides.pdf_url)
              .map((slides) => (
                <iframe
                  key={`${slides.id}:${slides.language}`}
                  className="material-slides-preview"
                  src={slides.pdf_url!}
                  title={`${t('Seminar slides')} · ${languageName(slides.language)} · ${slides.id}`}
                  loading="lazy"
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
      {materials.visualizations.length ? (
        <>
          <h3>{t('Interactive visualizations')}</h3>
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
      {materials.formalization_requests?.length ? (
        <>
          <h3>{t('Formalization requests')}</h3>
          <p>
            {t(
              'Join the shared proof work on GitHub. Each request describes the result and its current progress.',
            )}
          </p>
          <ul>
            {materials.formalization_requests.map((request) => (
              <li key={request.url}>
                <ExternalLink href={request.url}>{request.label}</ExternalLink>
              </li>
            ))}
          </ul>
        </>
      ) : null}
      <p>
        <ExternalLink href={materials.site_url}>{t('Reading and materials')}</ExternalLink>
        {' · '}
        <ExternalLink href={materials.repository_url}>{t('Repository')}</ExternalLink>
      </p>
    </section>
  );
}
