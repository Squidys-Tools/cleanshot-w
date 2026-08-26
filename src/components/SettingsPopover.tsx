type Props = {
  hotkey: string;
  includeCursor: boolean;
  launchAtStartup: boolean;
  saving: boolean;
  error: string | null;
  onSave: (hotkey: string, includeCursor: boolean, launchAtStartup: boolean) => void;
  onClose: () => void;
};

export default function SettingsPopover({
  hotkey,
  includeCursor,
  launchAtStartup,
  saving,
  error,
  onSave,
  onClose,
}: Props) {
  return (
    <div className="settings-popover" role="dialog" aria-labelledby="settings-title">
      <div className="settings-head">
        <div>
          <h2 id="settings-title">Capture settings</h2>
        </div>
        <button className="icon-btn" onClick={onClose} aria-label="Close settings">
          ×
        </button>
      </div>
      <form
        onSubmit={(event) => {
          event.preventDefault();
          const data = new FormData(event.currentTarget);
          const value = data.get("captureHotkey");
          if (typeof value === "string") {
            onSave(value, data.get("includeCursor") === "on", data.get("launchAtStartup") === "on");
          }
        }}
      >
        <label className="settings-field">
          <span>Global capture shortcut</span>
          <input name="captureHotkey" defaultValue={hotkey} autoComplete="off" spellCheck={false} />
          <small>Examples: Ctrl+Shift+4 or Alt+Space</small>
        </label>
        <label className="settings-check">
          <input name="includeCursor" type="checkbox" defaultChecked={includeCursor} />
          <span>
            <strong>Include cursor</strong>
            <small>Draw the current pointer into native screen and window captures.</small>
          </span>
        </label>
        <label className="settings-check">
          <input name="launchAtStartup" type="checkbox" defaultChecked={launchAtStartup} />
          <span>
            <strong>Launch at Windows startup</strong>
            <small>Starts CleanShot W minimized to the system tray.</small>
          </span>
        </label>
        {error && <p className="settings-error" role="alert">{error}</p>}
        <div className="settings-actions">
          <button type="button" className="btn" onClick={onClose}>
            Cancel
          </button>
          <button type="submit" className="btn primary" disabled={saving}>
            {saving ? "Saving…" : "Save settings"}
          </button>
        </div>
      </form>
    </div>
  );
}
