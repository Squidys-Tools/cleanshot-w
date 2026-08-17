type Props = {
  hotkey: string;
  saving: boolean;
  error: string | null;
  onSave: (hotkey: string) => void;
  onClose: () => void;
};

export default function SettingsPopover({ hotkey, saving, error, onSave, onClose }: Props) {
  return (
    <div className="settings-popover" role="dialog" aria-labelledby="settings-title">
      <div className="settings-head">
        <div>
          <span className="eyebrow">Preferences</span>
          <h2 id="settings-title">Capture settings</h2>
        </div>
        <button className="icon-btn" onClick={onClose} aria-label="Close settings">×</button>
      </div>
      <form onSubmit={(event) => {
        event.preventDefault();
        const data = new FormData(event.currentTarget);
        const value = data.get("captureHotkey");
        if (typeof value === "string") onSave(value);
      }}>
        <label className="settings-field">
          <span>Global capture shortcut</span>
          <input name="captureHotkey" defaultValue={hotkey} autoComplete="off" spellCheck={false} />
          <small>Examples: Ctrl+Shift+4 or Alt+Space</small>
        </label>
        {error && <p className="settings-error" role="alert">{error}</p>}
        <div className="settings-actions">
          <button type="button" className="btn" onClick={onClose}>Cancel</button>
          <button type="submit" className="btn primary" disabled={saving}>{saving ? "Saving…" : "Save shortcut"}</button>
        </div>
      </form>
    </div>
  );
}
