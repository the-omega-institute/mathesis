import { t } from '../i18n/locale';

export function GitHubLink({ compact = false, label = 'GitHub' }) {
  return (
    <a
      href="https://github.com/the-omega-institute/mathesis"
      className={`github-link${compact ? ' github-link-compact' : ''}`}
      aria-label={compact ? t('Mathesis on GitHub') : undefined}
      title={compact ? t('Mathesis on GitHub') : undefined}
    >
      <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
        <path d="M12 .75a11.25 11.25 0 0 0-3.558 21.923c.563.104.769-.244.769-.542 0-.267-.01-.975-.016-1.913-3.13.68-3.791-1.508-3.791-1.508-.512-1.3-1.25-1.646-1.25-1.646-1.022-.698.078-.684.078-.684 1.13.08 1.725 1.16 1.725 1.16 1.005 1.723 2.638 1.225 3.28.937.102-.728.393-1.225.715-1.507-2.499-.284-5.126-1.25-5.126-5.565 0-1.23.44-2.233 1.16-3.021-.116-.285-.503-1.43.11-2.98 0 0 .945-.303 3.094 1.155A10.78 10.78 0 0 1 12 6.18c.956.004 1.918.13 2.817.379 2.148-1.458 3.091-1.155 3.091-1.155.615 1.55.228 2.695.112 2.98.722.788 1.159 1.79 1.159 3.021 0 4.327-2.63 5.278-5.136 5.557.404.35.765 1.044.765 2.104 0 1.519-.014 2.744-.014 3.116 0 .3.203.65.774.54A11.25 11.25 0 0 0 12 .75Z" />
      </svg>
      {compact ? null : t(label)}
    </a>
  );
}
