import type { NativeWindowInfo } from "../lib/nativeCapture";

type Props = {
  windows: NativeWindowInfo[];
  busy: boolean;
  error: string | null;
  onSelect: (windowId: string) => void;
  onClose: () => void;
};

export default function WindowPicker({ windows, busy, error, onSelect, onClose }: Props) {
  return (
    <div className="window-picker-backdrop" role="presentation" onMouseDown={onClose}>
      <section className="window-picker" role="dialog" aria-modal="true" aria-labelledby="window-picker-title" onMouseDown={(event) => event.stopPropagation()}>
        <div className="window-picker-head">
          <div>
            <span className="eyebrow">Capture</span>
            <h2 id="window-picker-title">Choose a window</h2>
          </div>
          <button className="icon-btn" onClick={onClose} aria-label="Close window picker">×</button>
        </div>
        {busy && <p className="window-picker-message">Looking for open windows…</p>}
        {!busy && error && <p className="window-picker-message error">{error}</p>}
        {!busy && !error && windows.length === 0 && <p className="window-picker-message">No titled windows are available.</p>}
        {!busy && !error && windows.length > 0 && (
          <div className="window-picker-list">
            {windows.map((item) => (
              <button className="window-picker-item" key={item.id} onClick={() => onSelect(item.id)} disabled={busy}>
                <span className="window-picker-preview" aria-hidden="true" />
                <span className="window-picker-copy">
                  <strong>{item.title}</strong>
                  <small>{item.width} × {item.height}</small>
                </span>
                <span className="window-picker-arrow" aria-hidden="true">↗</span>
              </button>
            ))}
          </div>
        )}
        <p className="window-picker-hint">The window frame is included when Windows reports it as part of the bounds.</p>
      </section>
    </div>
  );
}
