import { useState } from "react";

type Props = {
  onFile: (blob: Blob, name: string) => void;
  onPick: () => void;
  onReadClipboard: () => Promise<void>;
  busy: boolean;
  notice: string | null;
};

export default function Dropzone({ onFile, onPick, onReadClipboard, busy, notice }: Props) {
  const [over, setOver] = useState(false);
  return (
    <div
      className={`dropzone ${over ? "over" : ""}`}
      onDragOver={(e) => {
        e.preventDefault();
        setOver(true);
      }}
      onDragLeave={() => setOver(false)}
      onDrop={(e) => {
        e.preventDefault();
        setOver(false);
        const f = e.dataTransfer.files[0];
        if (f && f.type.startsWith("image/")) onFile(f, f.name);
      }}
    >
      <div className="dz-icon">▣</div>
      <h2>{busy ? "Loading…" : "New capture"}</h2>
      <p>
        Paste a screenshot with <kbd>Ctrl</kbd>+<kbd>V</kbd>, drag an image in, or pick a file. Real screen capture
        arrives with the native shell.
      </p>
      <div className="dz-actions">
        <button className="btn primary" onClick={onPick} disabled={busy}>
          Pick image file
        </button>
        <button className="btn" onClick={onReadClipboard} disabled={busy}>
          Paste from clipboard…
        </button>
      </div>
      {notice && <p className="dz-notice">{notice}</p>}
    </div>
  );
}
