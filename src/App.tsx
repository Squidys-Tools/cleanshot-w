import { useEffect, useMemo, useRef, useState } from "react";
import {
  Archive,
  ArrowUpRight,
  Bookmark,
  CircleHelp,
  Clock3,
  Command,
  ExternalLink,
  FileText,
  Filter,
  Grid2X2,
  Image as ImageIcon,
  Layers3,
  Link2,
  List,
  Menu,
  MoreHorizontal,
  PanelRight,
  Plus,
  Search,
  Settings2,
  Sparkles,
  X,
} from "lucide-react";
import {
  assetUrl,
  createUrl,
  createNote,
  initializeStorage,
  isTauriRuntime,
  listActiveItems,
  saveFile,
  searchItems,
  type StoredLibraryItem,
} from "./lib/libraryApi";
import { classifyFile } from "./lib/ingestion/file-classification";
import "./App.css";

type ItemKind = "Article" | "Image" | "Note" | "PDF" | "Quote" | "Video" | "File";

type LibraryItem = {
  id: string | number;
  kind: ItemKind;
  title: string;
  description: string;
  source: string;
  date: string;
  tags: string[];
  ocrText?: string;
  image?: string;
  accent?: string;
  featured?: boolean;
  favorite?: boolean;
};

function displayKind(kind: string): ItemKind {
  switch (kind.toLowerCase()) {
    case "article":
    case "url":
      return "Article";
    case "image":
      return "Image";
    case "note":
      return "Note";
    case "pdf":
      return "PDF";
    case "video":
    case "embed":
      return "Video";
    case "file":
      return "File";
    default:
      return "Note";
  }
}

function formatItemDate(timestamp: number) {
  const date = new Date(timestamp);
  const age = Date.now() - date.getTime();
  if (age < 60 * 60 * 1000) return "Just now";
  if (age < 24 * 60 * 60 * 1000) return "Today";
  return date.toLocaleDateString(undefined, { month: "short", day: "2-digit" });
}

async function storedItemToLibraryItem(item: StoredLibraryItem): Promise<LibraryItem> {
  const kind = displayKind(item.kind);
  const metadataTags = item.metadata.tags;
  const tags = Array.isArray(metadataTags)
    ? metadataTags.filter((tag): tag is string => typeof tag === "string")
    : [];

  const image = await assetUrl(item.thumbnailPath ?? item.localAssetPath);

  return {
    id: item.id,
    kind,
    title: item.title?.trim() || "Untitled note",
    description:
      item.description?.trim() ||
      item.ocrText?.trim().slice(0, 180) ||
      "Saved to your mind.",
    source: item.sourceLabel || item.sourceUrl || "Quick note",
    date: formatItemDate(item.createdAt),
    tags,
    ocrText: item.ocrText,
    image,
    favorite: item.favorite,
  };
}

const seedItems: LibraryItem[] = [
  {
    id: 1,
    kind: "Article",
    title: "The quiet architecture of attention",
    description:
      "A field note on designing environments that make room for deep work, wandering, and the occasional useful distraction.",
    source: "thecreativeindependent.com",
    date: "Saved today",
    tags: ["attention", "writing"],
    accent: "ink",
    featured: true,
    favorite: true,
  },
  {
    id: 2,
    kind: "Image",
    title: "A room that remembers",
    description: "Warm light, timber, and one very good chair.",
    source: "are.na",
    date: "Yesterday",
    tags: ["interiors", "warm"],
    image:
      "https://images.unsplash.com/photo-1497366754035-f200968a6e72?auto=format&fit=crop&w=1000&q=85",
    favorite: true,
  },
  {
    id: 3,
    kind: "Note",
    title: "Things worth making time for",
    description:
      "A short list: morning pages, long walks without a destination, learning the names of trees, sending the postcard.",
    source: "Quick note",
    date: "Monday",
    tags: ["thoughts", "life"],
    accent: "paper-blue",
  },
  {
    id: 4,
    kind: "Image",
    title: "Orange as a signal",
    description: "A color study collected from a passing afternoon.",
    source: "Are.na channel",
    date: "May 18",
    tags: ["color", "reference"],
    image:
      "https://images.unsplash.com/photo-1518005020951-eccb494ad742?auto=format&fit=crop&w=1000&q=85",
  },
  {
    id: 5,
    kind: "PDF",
    title: "Notes on a living archive",
    description: "A small research paper on memory, retrieval, and why indexes become places.",
    source: "Internet Archive",
    date: "May 14",
    tags: ["research", "archive"],
    accent: "paper-green",
  },
  {
    id: 6,
    kind: "Quote",
    title: "“The mind is a place with weather.”",
    description: "— Annie Dillard",
    source: "Pilgrim at Tinker Creek",
    date: "May 08",
    tags: ["writing", "wonder"],
    accent: "paper-yellow",
  },
  {
    id: 7,
    kind: "Image",
    title: "Built for looking slowly",
    description: "A study in quiet proportions and imperfect repetition.",
    source: "mymind library",
    date: "May 02",
    tags: ["objects", "form"],
    image:
      "https://images.unsplash.com/photo-1549490349-8643362247b5?auto=format&fit=crop&w=1000&q=85",
  },
];

const spaces = [
  { name: "Design references", count: 38, color: "orange" },
  { name: "Read slowly", count: 24, color: "green" },
  { name: "Ideas in progress", count: 17, color: "blue" },
];

function KindIcon({ kind }: { kind: ItemKind }) {
  const Icon =
    kind === "Image"
      ? ImageIcon
      : kind === "Article"
        ? Link2
        : kind === "PDF"
          ? FileText
          : kind === "Quote"
            ? Bookmark
            : Sparkles;
  return <Icon size={13} strokeWidth={1.8} />;
}

function App() {
  const [items, setItems] = useState<LibraryItem[]>(isTauriRuntime() ? [] : seedItems);
  const [query, setQuery] = useState("");
  const [activeView, setActiveView] = useState("Everything");
  const [isAdding, setIsAdding] = useState(false);
  const [captureMode, setCaptureMode] = useState<"note" | "url" | "file">("note");
  const [newTitle, setNewTitle] = useState("");
  const [captureUrl, setCaptureUrl] = useState("");
  const [selectedFile, setSelectedFile] = useState<File | null>(null);
  const [selectedItem, setSelectedItem] = useState<LibraryItem | null>(null);
  const [listMode, setListMode] = useState(false);
  const [captureError, setCaptureError] = useState<string | null>(null);
  const searchRef = useRef<HTMLInputElement>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "/" && document.activeElement?.tagName !== "INPUT") {
        event.preventDefault();
        searchRef.current?.focus();
      }
      if (event.key === "Escape") {
        setSelectedItem(null);
        setIsAdding(false);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  useEffect(() => {
    if (!isTauriRuntime()) return;
    let cancelled = false;

    async function loadItems() {
      try {
        await initializeStorage();
        const storedItems = query.trim() ? await searchItems(query) : await listActiveItems();
        if (!cancelled) setItems(await Promise.all(storedItems.map(storedItemToLibraryItem)));
      } catch (error) {
        if (!cancelled) setCaptureError(error instanceof Error ? error.message : String(error));
      }
    }

    void loadItems();
    return () => {
      cancelled = true;
    };
  }, [query]);

  const filteredItems = useMemo(() => {
    const normalizedQuery = query.trim().toLowerCase();
    return items.filter((item) => {
      const matchesQuery = !normalizedQuery
        ? true
        : [item.title, item.description, item.source, item.kind, ...item.tags]
            .join(" ")
            .toLowerCase()
            .includes(normalizedQuery);
      const matchesView =
        activeView === "Everything" ||
        (activeView === "Top of mind" && item.favorite) ||
        (activeView === "Read later" && item.tags.includes("research")) ||
        (activeView === "Design references" && item.tags.includes("reference"));
      return matchesQuery && matchesView;
    });
  }, [activeView, items, query]);

  async function saveCapture(event: React.FormEvent) {
    event.preventDefault();
    setCaptureError(null);

    try {
      if (captureMode === "note") {
        if (!newTitle.trim()) return;
        if (isTauriRuntime()) {
          const storedItem = await createNote({
            body: newTitle.trim(),
            metadata: { captureSource: "quick-note" },
          });
          const libraryItem = await storedItemToLibraryItem(storedItem);
          setItems((current) => [libraryItem, ...current]);
        } else {
          const item: LibraryItem = {
            id: Date.now(),
            kind: "Note",
            title: newTitle.trim(),
            description: "Saved just now. Expand this thought whenever it asks for more room.",
            source: "Quick note",
            date: "Just now",
            tags: ["new note"],
            accent: "paper-blue",
          };
          setItems((current) => [item, ...current]);
        }
      } else if (captureMode === "url") {
        if (!captureUrl.trim()) return;
        const { ingestUrl } = await import("./lib/ingestion");
        const article = await ingestUrl(captureUrl.trim());
        const metadata = {
          author: article.author,
          publishedDate: article.publishedDate,
          extractedText: article.text,
          html: article.html,
          imageUrls: article.imageUrls,
          safeEmbeds: article.safeEmbeds,
          extractor: article.extractor,
        };

        if (isTauriRuntime()) {
          const storedItem = await createUrl({
            sourceUrl: article.canonicalUrl,
            title: article.title,
            description: article.description,
            body: article.text,
            metadata,
          });
          const libraryItem = await storedItemToLibraryItem(storedItem);
          setItems((current) => [libraryItem, ...current]);
        } else {
          const item: LibraryItem = {
            id: Date.now(),
            kind: "Article",
            title: article.title,
            description: article.description || article.text.slice(0, 180),
            source: new URL(article.canonicalUrl).hostname,
            date: "Just now",
            tags: [],
            image: article.imageUrls[0],
          };
          setItems((current) => [item, ...current]);
        }
      } else {
        if (!selectedFile) return;
        const kind = classifyFile(selectedFile);
        const bytes = Array.from(new Uint8Array(await selectedFile.arrayBuffer()));
        if (isTauriRuntime()) {
          const storedItem = await saveFile({
            fileName: selectedFile.name,
            mimeType: selectedFile.type,
            kind,
            bytes,
          });
          const libraryItem = await storedItemToLibraryItem(storedItem);
          setItems((current) => [libraryItem, ...current]);
        } else {
          const item: LibraryItem = {
            id: Date.now(),
            kind: kind === "image" ? "Image" : kind === "pdf" ? "PDF" : kind === "video" ? "Video" : "File",
            title: selectedFile.name,
            description: "Saved locally from this device.",
            source: "Local file",
            date: "Just now",
            tags: [],
            image: kind === "image" ? URL.createObjectURL(selectedFile) : undefined,
          };
          setItems((current) => [item, ...current]);
        }
      }

      setNewTitle("");
      setCaptureUrl("");
      setSelectedFile(null);
      setIsAdding(false);
    } catch (error) {
      setCaptureError(error instanceof Error ? error.message : String(error));
    }
  }

  function selectCaptureMode(mode: "note" | "url" | "file") {
    setCaptureMode(mode);
    setCaptureError(null);
  }

  function openCapture(mode: "note" | "url" | "file" = "note") {
    selectCaptureMode(mode);
    setIsAdding(true);
  }

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand-lockup">
          <div className="brand-mark" aria-hidden="true">
            <span />
          </div>
          <div>
            <strong>mymind</strong>
            <span>library</span>
          </div>
        </div>

        <nav className="primary-nav" aria-label="Main navigation">
          <button className="nav-item active" onClick={() => setActiveView("Everything")}>
            <Layers3 size={17} />
            <span>Everything</span>
            <span className="nav-count">{items.length}</span>
          </button>
          <button className="nav-item" onClick={() => setActiveView("Top of mind")}>
            <Sparkles size={17} />
            <span>Top of mind</span>
          </button>
          <button className="nav-item" onClick={() => setActiveView("Serendipity")}>
            <Clock3 size={17} />
            <span>Serendipity</span>
          </button>
          <button className="nav-item" onClick={() => setActiveView("Archive")}>
            <Archive size={17} />
            <span>Archive</span>
          </button>
        </nav>

        <div className="sidebar-section">
          <div className="sidebar-heading">
            <span>Spaces</span>
            <button className="icon-button small" aria-label="Add a Space" title="Add a Space">
              <Plus size={15} />
            </button>
          </div>
          <div className="space-list">
            {spaces.map((space) => (
              <button
                className="space-item"
                key={space.name}
                onClick={() => setActiveView(space.name)}
              >
                <span className={`space-dot ${space.color}`} />
                <span>{space.name}</span>
                <span className="space-count">{space.count}</span>
              </button>
            ))}
          </div>
        </div>

        <div className="sidebar-footer">
          <div className="sidebar-prompt">
            <span className="prompt-orb"><Sparkles size={14} /></span>
            <span>Save something<br />to your mind.</span>
          </div>
          <button className="nav-item footer-item">
            <Settings2 size={17} />
            <span>Settings</span>
          </button>
          <button className="nav-item footer-item">
            <CircleHelp size={17} />
            <span>Help & shortcuts</span>
          </button>
        </div>
      </aside>

      <main className="main-content">
        <header className="topbar">
          <button className="mobile-menu" aria-label="Open navigation"><Menu size={19} /></button>
          <div className="breadcrumb"><span>Library</span><span className="slash">/</span><strong>{activeView}</strong></div>
          <div className="topbar-actions">
            <button className="icon-button" aria-label="Filter library" title="Filter library"><Filter size={17} /></button>
            <button className="icon-button" aria-label="Open panel" title="Open panel"><PanelRight size={17} /></button>
            <div className="avatar">M</div>
          </div>
        </header>

        <section className="library-header">
          <div>
            <h1>{activeView === "Everything" ? "Everything" : activeView}</h1>
            <p><span className="live-dot" />{items.length} things saved · Search by whatever you remember.</p>
          </div>
          <button className="quiet-link" onClick={() => openCapture("note")}><Plus size={15} /> Add a note</button>
        </section>

        <section className="capture-bar" aria-label="Capture and search">
          <div className="search-field">
            <Search size={19} />
            <input
              ref={searchRef}
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder="Search your mind"
              aria-label="Search your mind"
            />
            <kbd><span>/</span> to search</kbd>
          </div>
          <button className="add-button" onClick={() => setIsAdding((current) => !current)}>
            <Plus size={18} />
            <span>Add to your mind</span>
          </button>
        </section>

        {isAdding && (
          <form className="quick-capture" onSubmit={saveCapture}>
            <div className="quick-capture-icon"><Sparkles size={16} /></div>
            <div className="capture-mode-tabs" role="tablist" aria-label="Capture type">
              <button type="button" className={captureMode === "note" ? "selected" : ""} onClick={() => selectCaptureMode("note")}>Note</button>
              <button type="button" className={captureMode === "url" ? "selected" : ""} onClick={() => selectCaptureMode("url")}>Link</button>
              <button type="button" className={captureMode === "file" ? "selected" : ""} onClick={() => selectCaptureMode("file")}>File</button>
            </div>
            {captureMode === "note" && <input
              autoFocus
              value={newTitle}
              onChange={(event) => setNewTitle(event.target.value)}
              placeholder="A thought, a link, a small beginning…"
              aria-label="New note"
            />}
            {captureMode === "url" && <input
              autoFocus
              type="url"
              value={captureUrl}
              onChange={(event) => setCaptureUrl(event.target.value)}
              placeholder="Paste a link to save and read later…"
              aria-label="URL to save"
            />}
            {captureMode === "file" && <>
              <input
                ref={fileInputRef}
                type="file"
                className="visually-hidden"
                accept="image/*,application/pdf,video/*"
                onChange={(event) => setSelectedFile(event.target.files?.[0] ?? null)}
              />
              <button type="button" className="file-picker" onClick={() => fileInputRef.current?.click()}>
                {selectedFile ? selectedFile.name : "Choose an image, PDF, or video"}
              </button>
            </>}
            <span className="capture-type">{captureMode === "note" ? "Quick note" : captureMode === "url" ? "Defuddle article" : "Local file"}</span>
            <button className="capture-save" type="submit">Save</button>
            <button className="capture-close" type="button" onClick={() => setIsAdding(false)} aria-label="Close capture"><X size={16} /></button>
          </form>
        )}

        {captureError && <p className="capture-error">Couldn’t save this yet: {captureError}</p>}

        <div className="library-toolbar">
          <div className="result-context">
            <span className="result-count">{filteredItems.length}</span> things to remember
            {query && <span className="search-context">for “{query}”</span>}
          </div>
          <div className="view-controls" aria-label="View options">
            <button className={`view-button ${!listMode ? "selected" : ""}`} onClick={() => setListMode(false)} aria-label="Grid view" title="Grid view"><Grid2X2 size={16} /></button>
            <button className={`view-button ${listMode ? "selected" : ""}`} onClick={() => setListMode(true)} aria-label="List view" title="List view"><List size={16} /></button>
          </div>
        </div>

        <div className={`library-grid ${listMode ? "list-mode" : ""}`}>
          {filteredItems.map((item, index) => (
            <article
              className={`library-card ${item.featured ? "featured-card" : ""} ${item.accent ?? ""}`}
              key={item.id}
              style={{ "--card-index": index } as React.CSSProperties}
              onClick={() => setSelectedItem(item)}
              tabIndex={0}
              onKeyDown={(event) => event.key === "Enter" && setSelectedItem(item)}
            >
              {item.image ? (
                <div className="card-image-wrap">
                  <img src={item.image} alt="" className="card-image" />
                  <button className="card-action" onClick={(event) => event.stopPropagation()} aria-label="More actions"><MoreHorizontal size={17} /></button>
                </div>
              ) : (
                <div className="card-paper-art" aria-hidden="true">
                  {item.kind === "Article" && <><span className="paper-line line-one" /><span className="paper-line line-two" /><span className="paper-seal">m</span></>}
                  {item.kind === "Note" && <><span className="note-scribble">remember<br />the shape<br />of a day</span><span className="note-star">✳</span></>}
                  {item.kind === "PDF" && <><span className="pdf-label">FIELD<br />NOTES</span><span className="pdf-rule" /></>}
                  {item.kind === "Quote" && <><span className="quote-mark">“</span><span className="quote-line" /></>}
                </div>
              )}
              <div className="card-content">
                <div className="card-kicker"><span><KindIcon kind={item.kind} />{item.kind}</span><span>{item.date}</span></div>
                <h2>{item.title}</h2>
                <p>{item.description}</p>
                <div className="card-footer"><span className="card-source">{item.source}</span><ArrowUpRight size={15} /></div>
              </div>
            </article>
          ))}
        </div>

        {filteredItems.length === 0 && (
          <div className="empty-state">
            <div className="empty-icon"><Search size={20} /></div>
            <h2>Nothing surfaced yet.</h2>
            <p>Try another word, or save something new to your mind.</p>
            <button className="text-button" onClick={() => { setQuery(""); setActiveView("Everything"); }}>Clear search</button>
          </div>
        )}

        <footer className="main-footer"><span>mymind library</span><span>Save without organizing.</span><span className="footer-shortcut"><Command size={12} /> K to add</span></footer>
      </main>

      {selectedItem && (
        <aside className="item-inspector" aria-label="Selected item">
          <div className="inspector-top"><span>Item details</span><button className="icon-button small" onClick={() => setSelectedItem(null)} aria-label="Close details"><X size={16} /></button></div>
          {selectedItem.image ? <img src={selectedItem.image} alt="" className="inspector-image" /> : <div className={`inspector-art ${selectedItem.accent ?? "ink"}`}><KindIcon kind={selectedItem.kind} /><span>{selectedItem.kind}</span></div>}
          <div className="inspector-copy"><div className="card-kicker"><span><KindIcon kind={selectedItem.kind} />{selectedItem.kind}</span><span>{selectedItem.date}</span></div><h2>{selectedItem.title}</h2><p>{selectedItem.description}</p><div className="inspector-source"><span>Source</span><strong>{selectedItem.source}</strong></div><div className="tag-row">{selectedItem.tags.map((tag) => <span key={tag}>#{tag}</span>)}</div><button className="open-source"><ExternalLink size={15} /> Open original</button></div>
        </aside>
      )}
    </div>
  );
}

export default App;
