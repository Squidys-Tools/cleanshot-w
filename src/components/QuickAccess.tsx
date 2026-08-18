export type OcrStatus = "idle" | "running" | "done" | "error";

type Props = {
  onCopyImage: () => void;
  onCopyFile: () => void;
  onSavePng: () => void;
  onPin?: () => void;
  onOcr: () => void;
  onNew: () => void;
  onClose: () => void;
  ocrStatus: OcrStatus;
  ocrProgress?: number;
  copied: boolean;
  notice?: string | null;
};

export default function QuickAccess({
  onCopyImage,
  onCopyFile,
  onSavePng,
  onPin,
  onOcr,
  onNew,
  onClose,
  ocrStatus,
  ocrProgress,
  copied,
  notice,
}: Props) {
  return (
    <div className="quick-access">
      <button className="qa-btn" onClick={onCopyImage} title="Copy the flattened image">
        {copied ? "✓ Copied" : "Copy image"}
      </button>
      <button className="qa-btn" onClick={onCopyFile} title="Copy the PNG as a file">
        Copy file
      </button>
      <button className="qa-btn" onClick={onOcr} title="Recognize text with tesseract.js">
        {ocrStatus === "running"
          ? `OCR ${Math.round((ocrProgress ?? 0) * 100)}%`
          : ocrStatus === "done"
            ? "Re-run OCR"
            : "Copy text (OCR)"}
      </button>
      <button className="qa-btn" onClick={onSavePng} title="Download as PNG">
        Save PNG
      </button>
      {onPin && (
        <button className="qa-btn" onClick={onPin} title="Keep this capture visible above other windows">
          Pin
        </button>
      )}
      <span className="qa-divider" />
      <button className="qa-btn" onClick={onNew} title="Start a new capture">
        New
      </button>
      <button className="qa-btn danger" onClick={onClose} title="Close (autosave is on)">
        Close
      </button>
      {notice && <span className="qa-notice" role="status">{notice}</span>}
    </div>
  );
}
