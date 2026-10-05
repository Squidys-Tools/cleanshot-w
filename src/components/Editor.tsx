import type { CaptureDoc, RegionMode, TldrawState } from "../types";
import type { EditorController } from "../lib/tldrawDoc";
import TldrawCanvas from "./editor/TldrawCanvas";
import EditorErrorBoundary from "./editor/EditorErrorBoundary";

type EditorProps = {
  doc: CaptureDoc;
  imageUrl: string;
  onChange: (recordId: string, annotations: TldrawState) => void;
  controllerRef: { current: EditorController | null };
  onHistoryState: (state: { canUndo: boolean; canRedo: boolean }) => void;
  saveState: "saved" | "saving" | "error";
  saveError?: string | null;
  onRetrySave?: () => void;
  regionMode: RegionMode | null;
  onRegionModeChange: (mode: RegionMode | null) => void;
  onCrop: (recordId: string, imageBlob: Blob, image: { width: number; height: number }, annotations: TldrawState) => Promise<void>;
  onOcrRegion: (rect: { x: number; y: number; width: number; height: number }) => void;
  onRename: (title: string) => Promise<boolean>;
};

function Editor({
  doc,
  imageUrl,
  onChange,
  controllerRef,
  onHistoryState,
  saveState,
  saveError,
  onRetrySave,
  regionMode,
  onRegionModeChange,
  onCrop,
  onOcrRegion,
  onRename,
}: EditorProps) {
  return (
    <div className="editor">
      {/* tldraw contains its own render errors, so a throw inside the editor
          subtree used to surface as a silent blank canvas with the rest of the
          app still running. This states the failure instead. Keyed on the
          capture so switching captures clears a previous failure. */}
      <EditorErrorBoundary resetKey={doc.id}>
        <TldrawCanvas
          imageUrl={imageUrl}
          imgW={doc.image.width}
          imgH={doc.image.height}
          title={doc.title}
          initialDoc={doc.annotations}
          onChange={(annotations) => onChange(doc.id, annotations)}
          controllerRef={controllerRef}
          onHistoryState={onHistoryState}
          saveState={saveState}
          saveError={saveError}
          onRetrySave={onRetrySave}
          regionMode={regionMode}
          onRegionModeChange={onRegionModeChange}
          onCrop={(imageBlob, image, annotations) => onCrop(doc.id, imageBlob, image, annotations)}
          onOcrRegion={onOcrRegion}
          onRename={onRename}
        />
      </EditorErrorBoundary>
    </div>
  );
}

export default Editor;
