import type { CaptureDoc, TldrawState } from "../types";
import type { EditorController } from "../lib/tldrawDoc";
import TldrawCanvas from "./editor/TldrawCanvas";

type EditorProps = {
  doc: CaptureDoc;
  imageUrl: string;
  onChange: (recordId: string, annotations: TldrawState) => void;
  controllerRef: { current: EditorController | null };
  onHistoryState: (state: { canUndo: boolean; canRedo: boolean }) => void;
};

function Editor({ doc, imageUrl, onChange, controllerRef, onHistoryState }: EditorProps) {
  return (
    <div className="editor">
      <TldrawCanvas
        imageUrl={imageUrl}
        imgW={doc.image.width}
        imgH={doc.image.height}
        title={doc.title}
        initialDoc={doc.annotations}
        onChange={(annotations) => onChange(doc.id, annotations)}
        controllerRef={controllerRef}
        onHistoryState={onHistoryState}
      />
    </div>
  );
}

export default Editor;
