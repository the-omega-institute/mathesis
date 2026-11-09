import type { ApiClient } from '../api/client';
import type { StatementDetail } from '../api/types';

/** A historical result must never inherit the current version's target or receipts. */
export async function loadCollectedResult(
  api: ApiClient,
  record: string,
  claim: string,
  version: string | undefined,
  signal: AbortSignal,
) {
  if (!version) {
    const result = await api.getStatement(record, claim, { signal });
    const paper = await api.getCollectedPaper(
      record,
      result.publication ? String(result.publication.version) : undefined,
      { signal },
    );
    return { paper, result, current: true };
  }
  const [paper, knowledge] = await Promise.all([
    api.getCollectedPaper(record, version, { signal }),
    api.collectedKnowledge(record, version, { signal }),
  ]);
  const statement = knowledge.statements.find((item) => item.id === claim);
  if (
    paper.id !== record ||
    knowledge.work !== record ||
    paper.revision !== version ||
    String(knowledge.version) !== version ||
    !statement
  )
    throw new Error('This result is not available in the requested paper version.');
  const description = statement.title || statement.section;
  const label = description ? `${statement.name}: ${description}` : statement.name;
  try {
    const current = await api.getStatement(record, claim, { signal });
    if (
      String(current.publication?.version) === version &&
      current.claim.statement === statement.statement
    )
      return {
        paper,
        result: {
          ...current,
          claim: {
            ...current.claim,
            label: description ? label : current.claim.label,
          },
        },
        current: true,
      };
  } catch (error) {
    if (signal.aborted) throw error;
    // A removed current statement does not hide an independently public older version.
  }
  const result: StatementDetail = {
    record,
    title: paper.title,
    publication: { version: knowledge.version, claims_revision: 0 },
    claim: {
      id: statement.id,
      kind: statement.kind,
      role: 'main',
      label,
      statement: statement.statement,
      section: statement.section ?? undefined,
      depends_on: [],
    },
    macros: knowledge.macros,
    target: null,
    verified_attempts: [],
  };
  return { paper, result, current: false };
}
