import { useEffect, useMemo, useRef, useState } from "react";
import type { CaptureRecord } from "../types";
import { filterCaptureRecords, normalizeCaptureTitle } from "../lib/history";
import { countMarkups } from "../lib/tldrawDoc";

const fmt = new Intl.DateTimeFormat(undefined, { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" });

type Props = {
  records: CaptureRecord[];
  currentId: string | null;
  onOpen: (r: CaptureRecord) => void;
  onRename: (id: string, title: string) => Promise<boolean>;
  onDelete: (id: string) => void;
};

export default function HistoryRail({ records, currentId, onOpen, onRename, onDelete }: Props) {
  const [query, setQuery] = useState("");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [draft, setDraft] = useState("");
  const [savingId, setSavingId] = useState<string | null>(null);
  const [renameError, setRenameError] = useState<string | null>(null);
  const skipBlurRef = useRef(false);

  const urls = useMemo(() => {
    const m = new Map<string, string>();
    for (const r of records) m.set(r.id, URL.createObjectURL(r.thumbBlob));
    return m;
  }, [records]);

  const filteredRecords = useMemo(() => filterCaptureRecords(records, query), [records, query]);

  useEffect(() => {
    return () => urls.forEach((u) => URL.revokeObjectURL(u));
  }, [urls]);

  const startRename = (record: CaptureRecord) => {
    skipBlurRef.current = false;
    setEditingId(record.id);
    setDraft(record.title);
    setRenameError(null);
  };

  const cancelRename = () => {
    setEditingId(null);
    setDraft("");
    setRenameError(null);
  };

  const commitRename = async (record: CaptureRecord) => {
    if (savingId === record.id) return;
    const title = normalizeCaptureTitle(draft);
    if (!title) {
      setRenameError("Capture titles cannot be empty.");
      return;
    }
    if (title === record.title) {
      cancelRename();
      return;
    }

    setSavingId(record.id);
    setRenameError(null);
    try {
      if (await onRename(record.id, title)) {
        setEditingId(null);
        setDraft("");
      } else {
        setRenameError("Could not rename this capture.");
      }
    } finally {
      setSavingId(null);
    }
  };

  return (
    <aside className="history">
      <div className="history-head">
        <strong>History</strong>
        <span>{query.trim() ? `${filteredRecords.length}/${records.length}` : records.length}</span>
      </div>
      <label className="history-search">
        <span aria-hidden="true">⌕</span>
        <input
          type="search"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          placeholder="Search titles…"
          aria-label="Search capture titles"
        />
        {query && (
          <button type="button" className="history-search-clear" onClick={() => setQuery("")} aria-label="Clear history search">
            ×
          </button>
        )}
      </label>
      {renameError && <p className="history-error" role="alert">{renameError}</p>}
      <div className="history-list">
        {records.length === 0 && (
          <p className="history-empty">No captures yet. Paste (Ctrl+V), drop an image, or pick a file.</p>
        )}
        {records.length > 0 && filteredRecords.length === 0 && (
          <p className="history-empty">No captures match “{query.trim()}”.</p>
        )}
        {filteredRecords.map((r) => {
          const markups = countMarkups(r.annotations);
          const editing = editingId === r.id;
          return (
            <div key={r.id} className={`history-item ${r.id === currentId ? "active" : ""}`} onClick={() => onOpen(r)}>
              <img src={urls.get(r.id)} alt="" />
              <div className="history-meta">
                {editing ? (
                  <form
                    className="history-rename"
                    onSubmit={(event) => {
                      event.preventDefault();
                      void commitRename(r);
                    }}
                    onClick={(event) => event.stopPropagation()}
                  >
                    <input
                      className="history-title-input"
                      value={draft}
                      maxLength={500}
                      autoFocus
                      aria-label={`Title for ${r.title}`}
                      onChange={(event) => setDraft(event.target.value)}
                      onKeyDown={(event) => {
                        if (event.key === "Escape") {
                          event.preventDefault();
                          skipBlurRef.current = true;
                          cancelRename();
                        }
                      }}
                      onBlur={() => {
                        if (skipBlurRef.current) {
                          skipBlurRef.current = false;
                          return;
                        }
                        void commitRename(r);
                      }}
                      disabled={savingId === r.id}
                    />
                    <small>{savingId === r.id ? "Saving…" : "Enter to save · Esc to cancel"}</small>
                  </form>
                ) : (
                  <button
                    type="button"
                    className="history-title"
                    title="Rename capture"
                    onClick={(event) => {
                      event.stopPropagation();
                      startRename(r);
                    }}
                  >
                    {r.title}
                  </button>
                )}
                <span>
                  {fmt.format(r.updatedAt)} · {markups} mark{markups === 1 ? "" : "s"}
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
          );
        })}
      </div>
    </aside>
  );
}
