import { useCallback } from 'react';
import { useApi } from '../api/context';
import { useAsync } from '../api/useAsync';
import { Async, DateText } from './ui';
import { t } from '../i18n/locale';

/** Mounted only in the administrator queue, never the author interface. */
export function ProcessingStatus() {
  const api = useApi();
  const load = useCallback((signal: AbortSignal) => api.processingStatus({ signal }), [api]);
  const status = useAsync(load);
  return (
    <details className="section">
      <summary>{t('Processing queues')}</summary>
      <p>
        {t(
          'Internal paper production and external submissions have separate capacity. Waiting for a service does not change a paper’s decision.',
        )}
      </p>
      <button type="button" className="button button-quiet button-small" onClick={status.reload}>
        {t('Refresh')}
      </button>
      <Async state={status.state} onRetry={status.reload}>
        {(lanes) => (
          <div className="table-scroll">
            <table>
              <thead>
                <tr>
                  <th>{t('Queue')}</th>
                  <th>{t('Capacity')}</th>
                  <th>{t('Running')}</th>
                  <th>{t('Waiting')}</th>
                  <th>{t('Failed')}</th>
                  <th>{t('Recent work')}</th>
                </tr>
              </thead>
              <tbody>
                {lanes
                  .filter((lane) => lane.lane === 'review' || lane.lane === 'collection')
                  .map((lane) => (
                    <tr key={lane.lane}>
                      <th>
                        {t(
                          lane.lane === 'collection'
                            ? 'Internal paper production'
                            : 'External submissions',
                        )}
                      </th>
                      <td>{lane.workers.reduce((n, worker) => n + worker.capacity, 0)}</td>
                      <td>{lane.running}</td>
                      <td>
                        {lane.queued}
                        {lane.oldest_queued_at ? (
                          <p className="small">
                            {t('Oldest waiting:')} <DateText iso={lane.oldest_queued_at} />
                          </p>
                        ) : null}
                      </td>
                      <td>
                        {lane.failed}
                        <p className="small">
                          {t('Retries:')} {lane.retries}
                        </p>
                      </td>
                      <td>
                        {lane.last_success ? (
                          <p>
                            {t('Last completed:')} {lane.last_success.submission} ·{' '}
                            {lane.last_success.stage} · <DateText iso={lane.last_success.at} />
                          </p>
                        ) : (
                          <p>{t('No completed work recorded yet.')}</p>
                        )}
                        {lane.last_failure ? (
                          <p>
                            {t('Needs attention:')} {lane.last_failure.submission} ·{' '}
                            {lane.last_failure.stage}: {lane.last_failure.reason}
                          </p>
                        ) : null}
                      </td>
                    </tr>
                  ))}
              </tbody>
            </table>
          </div>
        )}
      </Async>
    </details>
  );
}
