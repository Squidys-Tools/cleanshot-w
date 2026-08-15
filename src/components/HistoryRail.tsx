import { useEffect, useMemo } from "react";
import type { CaptureRecord } from "../types";
import { countMarkups } from "../lib/tldrawDoc";

const fmt = new Intl.DateTimeFormat(undefined, { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" });

type Props = {
  records: CaptureRecord[];
  currentId: string | null;
  onOpen: (r: CaptureRecord) => void;
  onDelete: (id: string) => void;
};

export default function HistoryRail({ records, currentId, onOpen, onDelete }: Props) {
  const urls = useMemo(() => {
    const m = new Map<string, string>();
    for (const r of records) m.set(r.id, URL.createObjectURL(r.thumbBlob));
    return m;
  }, [records]);

  useEffect(() => {
    return () => urls.forEach((u) => URL.revokeObjectURL(u));
  }, [urls]);

  return (
    <aside className="history">
      <div className="history-head">
        <strong>History</strong>
        <span>{records.length}</span>
      </div>
      <div className="history-list">
        {records.length === 0 && (
          <p className="history-empty">No captures yet. Paste (Ctrl+V), drop an image, or pick a file.</p>
        )}
        {records.map((r) => (
          <div key={r.id} className={`history-item ${r.id === currentId ? "active" : ""}`} onClick={() => onOpen(r)}>
            <img src={urls.get(r.id)} alt="" />
            <div className="history-meta">
              <strong>{r.title}</strong>
              <span>
                {fmt.format(r.updatedAt)} · {countMarkups(r.annotations)} mark{countMarkups(r.annotations) === 1 ? "" : "s"}
              </span>
            </div>
            <button
              className="icon-btn danger"
              title="Delete"
              onClick={(e) => {
                e.stopPropagation();
                onDelete(r.id);
              }}
            >
              ×
            </button>
          </div>
        ))}
      </div>
    </aside>
  );
}
