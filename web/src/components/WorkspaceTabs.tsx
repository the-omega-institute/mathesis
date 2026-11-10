export type WorkspaceSection = 'manuscript' | 'review' | 'formalization' | 'history' | 'advanced';

const LABELS: Record<WorkspaceSection, string> = {
  manuscript: 'Manuscript',
  review: 'Review',
  formalization: 'Formalization',
  history: 'Execution history',
  advanced: 'Advanced',
};

export function WorkspaceTabs({
  sections,
  active,
  onChange,
}: {
  sections: WorkspaceSection[];
  active: WorkspaceSection;
  onChange: (section: WorkspaceSection) => void;
}) {
  return (
    <div className="workspace-tabs" role="tablist" aria-label="Manuscript workspace">
      {sections.map((section, index) => (
        <button
          key={section}
          type="button"
          role="tab"
          id={`tab-${section}`}
          aria-selected={active === section}
          aria-controls="workspace-panel"
          tabIndex={active === section ? 0 : -1}
          onClick={() => onChange(section)}
          onKeyDown={(event) => {
            const next =
              event.key === 'ArrowRight'
                ? (index + 1) % sections.length
                : event.key === 'ArrowLeft'
                  ? (index + sections.length - 1) % sections.length
                  : event.key === 'Home'
                    ? 0
                    : event.key === 'End'
                      ? sections.length - 1
                      : undefined;
            if (next === undefined) return;
            event.preventDefault();
            const target = sections[next]!;
            onChange(target);
            document.getElementById(`tab-${target}`)?.focus();
          }}
        >
          {LABELS[section]}
        </button>
      ))}
    </div>
  );
}
