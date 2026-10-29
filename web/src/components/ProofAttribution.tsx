import type { FormalizationAttribution } from '../api/types';
import { SourceLink } from './ui';
export function ProofAttribution({ attribution }: { attribution?: FormalizationAttribution }) {
  if (!attribution) return null;
  return (
    <p className="small">
      Original authors: {attribution.authors.join(', ')} ·{' '}
      {attribution.formalizers?.length ? (
        <>Formalization: {attribution.formalizers.join(', ')} · </>
      ) : null}
      <SourceLink source={attribution.source_reference} /> · {attribution.license}
    </p>
  );
}
