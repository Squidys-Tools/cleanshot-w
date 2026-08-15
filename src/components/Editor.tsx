import type { CaptureDoc, TldrawState } from "../types";
import type { Exporter } from "../lib/tldrawDoc";
import TldrawCanvas from "./editor/TldrawCanvas";

type EditorProps = {
  doc: CaptureDoc;
  imageUrl: string;
  onChange: (recordId: string, annotations: TldrawState) => void;
  exportRef: { current: Exporter | null };
};

function Editor({ doc, imageUrl, onChange, exportRef }: EditorProps) {
  return (
    <div className="editor">
      <TldrawCanvas
        imageUrl={imageUrl}
        imgW={doc.image.width}
        imgH={doc.image.height}
        title={doc.title}
        initialDoc={doc.annotations}
        onChange={(annotations) => onChange(doc.id, annotations)}
        exportRef={exportRef}
      />
    </div>
  );
}

export default Editor;
