import { t } from '../i18n/locale';
export function PublicationChoice({
  checked,
  onChange,
}: {
  checked: boolean;
  onChange: (checked: boolean) => void;
}) {
  return (
    <div>
      <label className="checkbox">
        <input
          type="checkbox"
          checked={checked}
          onChange={(event) => onChange(event.target.checked)}
          aria-describedby="publication-choice-hint publication-choice-legacy-hint"
        />
        <span>{t('Make my results public once accepted.')}</span>
      </label>
      <p className="hint" id="publication-choice-hint">
        {t('An accepted paper is published under ')}
        <a href="https://creativecommons.org/licenses/by/4.0/">CC BY 4.0</a>
        {t(
          '. The repository and the archive record with its DOI are permanent and stay available if you later make the Mathesis page private. Reviews, letters and advice stay private.',
        )}
      </p>
      <p className="hint">
        {t(
          'Unchecking gives a private review without a publication decision. Your manuscript and report remain private.',
        )}
      </p>
      <span className="visually-hidden" id="publication-choice-legacy-hint">
        {t(
          'Uncheck this box if you only want a review. Your manuscript and report will remain private.',
        )}
      </span>
    </div>
  );
}
