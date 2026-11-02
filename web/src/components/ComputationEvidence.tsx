import { t } from '../i18n/locale';
import type { ComputationReceipt, RefereeRound } from '../api/types';

const states: Record<ComputationReceipt['state'], string> = {
  counterexample: 'Counterexample to the checked matrix inequality',
  not_counterexample: 'This witness is not a counterexample',
  invalid_input: 'Input is outside the supported certificate format',
  arithmetic_limit: 'Exact arithmetic limit reached',
};

export function ComputationEvidence({ round }: { round: RefereeRound }) {
  if (!round.computation_receipts?.length) return null;
  return (
    <section aria-label={t('Replayed finite computations')} className="report">
      <h3>{t('Replayed finite computations')}</h3>
      <p>
        {t(
          'These results check the specified finite calculation. The source audit must establish whether it applies to your statement.',
        )}
      </p>
      {round.computation_receipts.map((receipt) => (
        <details className="history" key={receipt.attachment.path}>
          <summary>
            {receipt.attachment.path} · {states[receipt.state]}
          </summary>
          {receipt.numerator !== null && receipt.denominator !== null ? (
            <p>
              {t('Exact quadratic form: ')}
              {receipt.numerator}/{receipt.denominator}
            </p>
          ) : null}
          <p>{receipt.explanation}</p>
          <p>
            {t('Declared claim references: ')}
            {receipt.attachment.claims.join(', ') || 'None'}
          </p>
          <p>
            {t('Source version ')}
            {receipt.version} {t(' · statements revision ')}
            {receipt.claims_revision}
          </p>
          <p>
            {t('Input SHA-256: ')}
            <code>{receipt.attachment.sha256}</code>
          </p>
          <p>
            {t('Calculation protocol: ')}
            {receipt.protocol}
          </p>
          <p>
            {t('Checked: ')}
            {receipt.checked_at}
          </p>
        </details>
      ))}
    </section>
  );
}
