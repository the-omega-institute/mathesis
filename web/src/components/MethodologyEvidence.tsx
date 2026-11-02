import { t } from '../i18n/locale';
import type { EvidenceStatus, MethodologyReading } from '../api/types';
import { SHAPE_LABELS } from '../lib/labels';
import { Markdown } from './Markdown';

const STATUS: Record<EvidenceStatus, string> = {
  supported: 'Supported by the reported evidence',
  unsupported: 'Not supported by the reported evidence',
  insufficient: 'Insufficient evidence',
};

export function MethodologyEvidence({ reading }: { reading: MethodologyReading }) {
  return (
    <details className="history">
      <summary>
        {t('New content and mathematical value · ')}
        {t(SHAPE_LABELS[reading.escape])}
      </summary>
      <p className="hint">
        {t(
          'Source-based reviewer judgments. Reported sources and computations are not independently verified proofs.',
        )}
      </p>
      <h4>{t('Prerequisites and comparison')}</h4>
      <p>
        {t('Literature: ')}
        {t(STATUS[reading.literature])}
      </p>
      <ul>
        {reading.baseline.map((item, index) => (
          <li key={index}>
            <Markdown text={item.statement} />
            <p>
              {t('Location: ')}
              {item.location}
            </p>
            {item.source ? <Markdown text={item.source} /> : null}
          </li>
        ))}
      </ul>
      <h4>{t('Search scope')}</h4>
      <ul>
        {reading.searched.map((scope, index) => (
          <li key={index}>{scope}</li>
        ))}
      </ul>
      <Markdown text={reading.binding_analysis} />
      <h4>{t('Additional mathematical content')}</h4>
      <Markdown text={reading.delta || t('No supported additional content identified.')} />
      {reading.witnesses.map((witness, index) => (
        <div key={index}>
          <p>
            {witness.kind === 'conclusion' ? t('Conclusion witness') : t('Intermediate witness')} ·{' '}
            {witness.location}
          </p>
          <Markdown text={witness.statement} />
          <Markdown text={witness.non_binding_step} />
          <p>{t('Use in the proof:')}</p>
          <Markdown text={witness.live_use} />
        </div>
      ))}
      <h4>{t('Mathematical value')}</h4>
      <p>
        {reading.utility.kind.replaceAll('_', ' ')} · {t(STATUS[reading.utility.status])}
      </p>
      <Markdown text={reading.utility.rationale} />
      {(
        [
          ['Question or assertion', reading.utility.problem],
          ['Progress or application', reading.utility.progress],
          ['Exact prerequisites', reading.utility.prerequisites],
          ['Discharge of prerequisites', reading.utility.discharge],
        ] as const
      )
        .filter(([, text]) => text)
        .map(([label, text]) => (
          <div key={label}>
            <strong>{t(label)}</strong>
            <Markdown text={text} />
          </div>
        ))}
      {reading.settlement ? (
        <div>
          <h4>{t('Named open-problem settlement')}</h4>
          <p>
            {reading.settlement.problem.name} · {t(STATUS[reading.settlement.status])}
          </p>
          {(
            [
              'statement',
              'assumptions',
              'prior_status',
              'resolution',
              'matching_argument',
              'opened_source',
            ] as const
          ).map((key) => (
            <div key={key}>
              <strong>{key.replaceAll('_', ' ')}</strong>
              <Markdown text={reading.settlement![key]} />
            </div>
          ))}
        </div>
      ) : null}
      {reading.information_gain ? (
        <div>
          <h4>{t('Separate information-gain analysis')}</h4>
          <p>
            {t(STATUS[reading.information_gain.status])}
            {t('. This analysis does not decide publication.')}
          </p>
          {(['model', 'readout', 'task', 'correspondence', 'evidence'] as const).map((key) => (
            <div key={key}>
              <strong>{key}</strong>
              <Markdown text={reading.information_gain![key]} />
            </div>
          ))}
        </div>
      ) : null}
      {reading.limits.length ? (
        <>
          <h4>{t('Limits and unresolved questions')}</h4>
          <ul>
            {reading.limits.map((limit, index) => (
              <li key={index}>{limit}</li>
            ))}
          </ul>
        </>
      ) : null}
    </details>
  );
}
