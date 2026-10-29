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
          aria-describedby="publication-choice-hint"
        />
        <span>Publish my manuscript publicly if accepted.</span>
      </label>
      <p className="hint" id="publication-choice-hint">
        If you only want review feedback, uncheck this box. Your manuscript and report will remain
        private.
      </p>
    </div>
  );
}
