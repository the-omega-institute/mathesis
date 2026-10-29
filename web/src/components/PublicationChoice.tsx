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
        <span>Make my results public once accepted.</span>
      </label>
      <p className="hint" id="publication-choice-hint">
        Uncheck this box if you only want a review. Your manuscript and report will remain private.
      </p>
    </div>
  );
}
