export type OcrStatus = "idle" | "running" | "done" | "error";

type Props = {
  onCopyImage: () => void;
  onSavePng: () => void;
  onOcr: () => void;
  onNew: () => void;
  onClose: () => void;
  ocrStatus: OcrStatus;
  ocrProgress?: number;
  copied: boolean;
};

export default function QuickAccess({
  onCopyImage,
  onSavePng,
  onOcr,
  onNew,
  onClose,
  ocrStatus,
  ocrProgress,
  copied,
}: Props) {
  return (
    <div className="quick-access">
      <button className="qa-btn" onClick={onCopyImage} title="Copy the flattened image">
        {copied ? "✓ Copied" : "Copy image"}
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
      <span className="qa-divider" />
      <button className="qa-btn" onClick={onNew} title="Start a new capture">
        New
      </button>
      <button className="qa-btn danger" onClick={onClose} title="Close (autosave is on)">
        Close
      </button>
    </div>
  );
}
