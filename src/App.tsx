import { useState, type ReactNode } from "react";
import "./App.css";

type CaptureMode = "area" | "window" | "fullscreen" | "scroll";

const captures = [
  { title: "Checkout flow", meta: "Today · 2:14 PM", tone: "violet" },
  { title: "API response", meta: "Today · 1:48 PM", tone: "blue" },
  { title: "Dashboard state", meta: "Yesterday · 4:32 PM", tone: "amber" },
];

function Icon({ name }: { name: string }) {
  const paths: Record<string, ReactNode> = {
    grid: <><rect x="3" y="3" width="7" height="7" rx="1" /><rect x="14" y="3" width="7" height="7" rx="1" /><rect x="3" y="14" width="7" height="7" rx="1" /><rect x="14" y="14" width="7" height="7" rx="1" /></>,
    clock: <><circle cx="12" cy="12" r="8.5" /><path d="M12 7v5l3.5 2" /></>,
    settings: <><path d="M12 8.2a3.8 3.8 0 1 0 0 7.6 3.8 3.8 0 0 0 0-7.6Z" /><path d="m19.4 15 .1.1a1.9 1.9 0 0 1-2.7 2.7l-.1-.1a1.9 1.9 0 0 0-3.2 1.3v.2a1.9 1.9 0 0 1-3.8 0V19a1.9 1.9 0 0 0-3.2-1.3l-.1.1a1.9 1.9 0 0 1-2.7-2.7l.1-.1A1.9 1.9 0 0 0 2.5 12a1.9 1.9 0 0 1 1.9-1.9h.2A1.9 1.9 0 0 0 5.9 7l-.1-.1a1.9 1.9 0 0 1 2.7-2.7l.1.1A1.9 1.9 0 0 0 12 3.5a1.9 1.9 0 0 1 1.9 1.9v.2A1.9 1.9 0 0 0 17 6.9l.1-.1a1.9 1.9 0 0 1 2.7 2.7l-.1.1A1.9 1.9 0 0 0 21.1 12a1.9 1.9 0 0 1-1.9 1.9H19a1.9 1.9 0 0 0 .4 1.1Z" /></>,
    area: <><path d="M4 9V5a1 1 0 0 1 1-1h4M15 4h3a2 2 0 0 1 2 2v3M20 15v3a2 2 0 0 1-2 2h-3M9 20H6a2 2 0 0 1-2-2v-3" /></>,
    window: <><rect x="3.5" y="4" width="17" height="16" rx="2" /><path d="M4 8.5h16M7 6.2h.1M10 6.2h.1" /></>,
    monitor: <><rect x="3" y="4" width="18" height="13" rx="2" /><path d="M8 21h8M12 17v4" /></>,
    scroll: <><path d="M7 4h10M7 20h10" /><rect x="5" y="4" width="14" height="16" rx="3" /><path d="M12 8v8m-2-2 2 2 2-2" /></>,
    search: <><circle cx="10.8" cy="10.8" r="6.2" /><path d="m16 16 4.5 4.5" /></>,
    plus: <><path d="M12 5v14M5 12h14" /></>,
    arrow: <><path d="M5 19 19 5M10 5h9v9" /></>,
    play: <path d="m9 6 9 6-9 6V6Z" />,
    more: <><circle cx="5" cy="12" r="1" fill="currentColor" stroke="none" /><circle cx="12" cy="12" r="1" fill="currentColor" stroke="none" /><circle cx="19" cy="12" r="1" fill="currentColor" stroke="none" /></>,
  };
  return <svg className="icon" viewBox="0 0 24 24" aria-hidden="true">{paths[name]}</svg>;
}

function App() {
  const [mode, setMode] = useState<CaptureMode>("area");
  const [capturing, setCapturing] = useState(false);
  const [searching, setSearching] = useState(false);

  const startCapture = () => {
    setCapturing(true);
    window.setTimeout(() => setCapturing(false), 900);
  };

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand"><span className="brand-mark"><span /></span><span>CleanShot <em>W</em></span></div>
        <nav className="primary-nav" aria-label="Primary navigation">
          <button className="nav-item active"><Icon name="grid" /> <span>Capture studio</span><kbd>⌘ 1</kbd></button>
          <button className="nav-item"><Icon name="clock" /> <span>History</span><span className="nav-count">12</span></button>
          <button className="nav-item"><Icon name="monitor" /> <span>Pinned</span></button>
        </nav>
        <div className="sidebar-bottom">
          <div className="storage-label"><span>Local library</span><strong>2.4 GB</strong></div>
          <div className="storage-track"><span /></div>
          <button className="nav-item"><Icon name="settings" /> <span>Preferences</span></button>
          <div className="profile"><div className="avatar">A</div><div><strong>Alex Morgan</strong><span>Local workspace</span></div><Icon name="more" /></div>
        </div>
      </aside>

      <main className="main-content">
        <header className="topbar">
          <div><p className="eyebrow">CAPTURE STUDIO</p><h1>Make the screen say exactly what you mean.</h1></div>
          <div className="top-actions"><button className="icon-button" aria-label="Search" onClick={() => setSearching(!searching)}><Icon name="search" /></button><button className="avatar small">A</button></div>
        </header>

        <section className="capture-panel">
          <div className="panel-heading"><div><h2>New capture</h2><p>Choose a mode, then use the shortcut anywhere.</p></div><span className="ready-status"><i /> Ready</span></div>
          <div className="mode-grid">
            {([['area', 'Area', 'Select any region', '⌘ ⇧ 4', 'area'], ['window', 'Window', 'Snap to an app', '⌘ ⇧ 5', 'window'], ['fullscreen', 'Fullscreen', 'Capture a display', '⌘ ⇧ 6', 'monitor'], ['scroll', 'Scrolling', 'Capture a long page', '⌘ ⇧ 7', 'scroll']] as const).map(([key, title, desc, shortcut, icon]) => (
              <button key={key} className={`mode-card ${mode === key ? 'selected' : ''}`} onClick={() => setMode(key)}><span className="mode-icon"><Icon name={icon} /></span><span className="mode-copy"><strong>{title}</strong><small>{desc}</small></span><kbd>{shortcut}</kbd></button>
            ))}
          </div>
          <div className="capture-footer"><span className="hint"><span className="hint-dot" /> Hover a window to preview its bounds</span><button className="capture-button" onClick={startCapture}><span className={capturing ? 'pulse' : ''}><Icon name="plus" /></span>{capturing ? 'Opening capture…' : 'Start capture'}<kbd>↵</kbd></button></div>
        </section>

        <section className="recent-section"><div className="section-heading"><div><h2>Recent captures</h2><p>Everything stays local until you choose to share it.</p></div><button className="text-button">View history <Icon name="arrow" /></button></div>
          <div className="capture-list">{captures.map((capture, index) => <article className="capture-row" key={capture.title}><div className={`capture-thumb ${capture.tone}`}><span className="thumb-bar" /><span className="thumb-line one" /><span className="thumb-line two" /><span className="thumb-box" />{index === 0 && <span className="thumb-arrow"><Icon name="arrow" /></span>}</div><div className="capture-info"><strong>{capture.title}</strong><span>{capture.meta}</span></div><span className="capture-type">PNG · {index === 0 ? '2.1 MB' : index === 1 ? '884 KB' : '1.4 MB'}</span><button className="row-action" aria-label={`Open ${capture.title}`}><Icon name="arrow" /></button></article>)}</div>
        </section>
        <footer className="statusbar"><span><span className="status-led" /> Local mode</span><span>Shortcut <kbd>⌘ ⇧ 4</kbd> <button className="status-link">Customize</button></span></footer>
      </main>
      {searching && <div className="search-popover"><Icon name="search" /><input autoFocus placeholder="Search captures…" onKeyDown={(event) => event.key === 'Escape' && setSearching(false)} /><kbd>Esc</kbd></div>}
    </div>
  );
}

export default App;
