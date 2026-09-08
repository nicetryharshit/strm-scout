import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open } from "@tauri-apps/plugin-dialog";
import { Check, ChevronLeft, ChevronRight, Download, FileImage, FolderOpen, Moon, Pause, Play, RotateCcw, Sun, X } from "lucide-react";
import type { FrameRange, StreamInfo } from "./types";

const frameBounds = (range: FrameRange | null, info: StreamInfo) => range && range.begin >= 0
  ? [Math.max(0, range.begin), Math.min(info.frameCount - 1, range.end)]
  : [0, info.frameCount - 1];

export default function App() {
  const [info, setInfo] = useState<StreamInfo | null>(null);
  const [selected, setSelected] = useState(0);
  const [checked, setChecked] = useState<number[]>([]);
  const [frame, setFrame] = useState(0);
  const [playing, setPlaying] = useState(false);
  const [speed, setSpeed] = useState(1);
  const [imageUrl, setImageUrl] = useState("");
  const [loading, setLoading] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [status, setStatus] = useState("Open an STRM file to begin");
  const [exportName, setExportName] = useState("");
  const [exportSeparator, setExportSeparator] = useState("_");
  const [exportStartNumber, setExportStartNumber] = useState("0");
  const [darkMode, setDarkMode] = useState(() => localStorage.getItem("strm-scout-theme") === "dark");
  const frameRequest = useRef(0);
  const frameBusy = useRef(false);

  useEffect(() => {
    localStorage.setItem("strm-scout-theme", darkMode ? "dark" : "light");
  }, [darkMode]);

  useEffect(() => {
    const preventContextMenu = (event: MouseEvent) => event.preventDefault();
    window.addEventListener("contextmenu", preventContextMenu);
    return () => window.removeEventListener("contextmenu", preventContextMenu);
  }, []);

  const currentRange = info?.ranges[selected] ?? null;
  const bounds = info ? frameBounds(currentRange, info) : [0, 0];

  const showFrame = useCallback(async (index: number) => {
    if (!info) return;
    if (frameBusy.current) return;
    frameBusy.current = true;
    const request = ++frameRequest.current;
    try {
      const data = await invoke<ArrayBuffer>("decode_frame", { index });
      if (request !== frameRequest.current) return;
      const next = URL.createObjectURL(new Blob([data], { type: "image/png" }));
      setImageUrl(previous => {
        if (previous) URL.revokeObjectURL(previous);
        return next;
      });
      setFrame(index);
    } catch (error) {
      setPlaying(false);
      setStatus(String(error));
    } finally {
      frameBusy.current = false;
    }
  }, [info]);

  const loadFile = useCallback(async (path: string) => {
    setLoading(true);
    setPlaying(false);
    setStatus("Reading stream and section index…");
    try {
      const loaded = await invoke<StreamInfo>("load_strm", { path });
      setInfo(loaded);
      setExportName(loaded.fileStem || "export");
      setExportSeparator("_");
      setExportStartNumber("0");
      setSelected(0);
      setChecked(loaded.ranges.map((_, index) => index));
      setFrame(0);
      setStatus(`${loaded.frameCount} frames ready`);
    } catch (error) {
      setInfo(null);
      setStatus(String(error));
    } finally {
      setLoading(false);
    }
  }, []);

  const openFile = async () => {
    const path = await open({ multiple: false, filters: [{ name: "STRM stream", extensions: ["strm"] }] });
    if (typeof path === "string") await loadFile(path);
  };

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let active = true;
    void getCurrentWebview().onDragDropEvent(event => {
      if (event.payload.type !== "drop") return;
      const path = event.payload.paths.find(candidate => candidate.toLowerCase().endsWith(".strm"));
      if (path) void loadFile(path);
      else setStatus("Drop a .strm file to open it");
    }).then(listener => {
      if (active) unlisten = listener;
      else listener();
    });
    return () => {
      active = false;
      unlisten?.();
    };
  }, [loadFile]);

  useEffect(() => {
    if (info) showFrame(frameBounds(info.ranges[0] ?? null, info)[0]);
  }, [info, showFrame]);

  useEffect(() => {
    if (!playing || !info) return;
    const delay = 1000 / (info.fps * speed);
    const timer = window.setInterval(() => {
      setFrame(previous => {
        const next = previous >= bounds[1] ? bounds[0] : previous + 1;
        showFrame(next);
        return next;
      });
    }, delay);
    return () => window.clearInterval(timer);
  }, [playing, info, speed, bounds[0], bounds[1], showFrame]);

  const selectRange = (index: number) => {
    if (!info) return;
    setPlaying(false);
    setSelected(index);
    const [begin] = frameBounds(info.ranges[index], info);
    showFrame(begin);
  };

  const toggleChecked = (index: number) => setChecked(value => value.includes(index)
    ? value.filter(item => item !== index)
    : [...value, index]);

  const exportRanges = async () => {
    if (!info || checked.length === 0) return;
    const directory = await open({ directory: true, multiple: false, title: "Choose export folder" });
    if (!directory) return;
    setExporting(true);
    setStatus("Exporting PNG sequences…");
    try {
      const result = await invoke<string>("export_ranges", {
        directory,
        rangeIndexes: checked,
        fileName: exportName || info.fileStem || "export",
        separator: exportSeparator || "_",
        startNumber: Number.parseInt(exportStartNumber, 10) || 0,
      });
      setStatus(result);
    } catch (error) {
      setStatus(String(error));
    } finally {
      setExporting(false);
    }
  };

  const exportFrame = async () => {
    if (!info) return;
    const directory = await open({ directory: true, multiple: false, title: "Choose export folder" });
    if (!directory) return;
    try {
      const result = await invoke<string>("export_frame", {
        directory,
        index: frame,
        fileName: exportName || info.fileStem || "export",
        separator: exportSeparator || "_",
        startNumber: Number.parseInt(exportStartNumber, 10) || 0,
      });
      setStatus(result);
    } catch (error) {
      setStatus(String(error));
    }
  };

  return <div className={`app-shell ${darkMode ? "dark" : ""}`}>
    <style>{`.viewer,.canvas-stage{background-color:#f3f3f3;background-image:linear-gradient(45deg,#fff 25%,transparent 25%),linear-gradient(-45deg,#fff 25%,transparent 25%),linear-gradient(45deg,transparent 75%,#fff 75%),linear-gradient(-45deg,transparent 75%,#fff 75%);background-size:24px 24px;background-position:0 0,0 12px,12px -12px,-12px 0}.inspector{overflow:hidden}.export-section{min-height:0;overflow:hidden}.export-section .check-list{max-height:220px;overflow-y:auto;padding-right:4px}.fake-check{flex:0 0 16px;background:#fff}.dark .sidebar,.dark .inspector,.dark .transport,.dark .file-card,.dark .empty-glyph,.dark .pattern-row input,.dark .transport-buttons button,.dark .transport select{background:#24272b;color:#f4f4f2}.dark .viewer,.dark .canvas-stage{background-color:#f3f3f3}.dark .app-mark{background:#f4f4f2;color:#17191c}.dark .fake-check{background:#fff;border-color:#f4f4f2;color:#17191c}.dark .check-list input:checked+.fake-check{background:#f4f4f2;color:#17191c}.dark .button{border-color:#596169;background:#24272b;color:#f4f4f2}.dark .button.primary,.dark .transport-buttons .play{background:#f4f4f2;border-color:#f4f4f2;color:#17191c}.dark .export-pattern,.dark .export-option,.dark .range-row.active{background:#30343a;border-color:#596169}.theme-toggle{margin-left:auto}.titlebar .theme-toggle+.button{margin-left:0}`}</style>
    <header className="titlebar">
      <div className="app-mark">S</div>
      <div><strong>STRM Scout</strong></div>
      <button className="button theme-toggle" onClick={() => setDarkMode(value => !value)} aria-label={darkMode ? "Use light mode" : "Use dark mode"}>{darkMode ? <Sun size={16} /> : <Moon size={16} />}{darkMode ? "Light" : "Dark"}</button>
      <button className="button primary" onClick={openFile}><FolderOpen size={16} /> Open file</button>
    </header>

    <main className="workspace">
      <aside className="sidebar">
        <div className="panel-heading"><span>Sections</span>{info && <em>{info.ranges.length}</em>}</div>
        <div className="range-list">
          {!info && <div className="empty-side">No section index loaded</div>}
          {info?.ranges.map((range, index) => {
            const [begin, end] = frameBounds(range, info);
            return <button className={`range-row ${selected === index ? "active" : ""}`} key={`${range.id}-${index}`} onClick={() => selectRange(index)}>
              <span className="range-icon"><FileImage size={15} /></span>
              <span className="range-copy"><strong>{range.name}</strong><small>{begin}–{end} · {end - begin + 1} frames</small></span>
              <ChevronRight size={15} />
            </button>;
          })}
        </div>
        {info && <div className="file-card">
          <span className="eyebrow">Loaded file</span>
          <strong>{info.fileName}</strong>
          <small>{(info.fileSize / 1048576).toFixed(2)} MB</small>
          <button className="icon-button" onClick={() => { setInfo(null); setImageUrl(""); setPlaying(false); }} aria-label="Close file"><X size={15} /></button>
        </div>}
      </aside>

      <section className="viewer">
        <div className="canvas-stage">
          {!info && <div className="empty-state"><div className="empty-glyph"><FileImage size={30} /></div><h1>Inspect an STRM animation</h1><p>Drop an STRM file anywhere in the window, or choose one to inspect sections and export PNG sequences.</p><button className="button primary" onClick={openFile}><FolderOpen size={16} /> Choose STRM file</button></div>}
          {loading && <div className="busy"><span className="spinner" />Reading stream…</div>}
          {info && imageUrl && <div className="image-checker"><img src={imageUrl} alt={`Frame ${frame}`} /></div>}
        </div>
        <div className="transport">
          <div className="transport-buttons">
            <button onClick={() => showFrame(Math.max(bounds[0], frame - 1))} disabled={!info}><ChevronLeft size={18} /></button>
            <button className="play" onClick={() => setPlaying(value => !value)} disabled={!info}>{playing ? <Pause size={18} /> : <Play size={18} fill="currentColor" />}</button>
            <button onClick={() => showFrame(Math.min(bounds[1], frame + 1))} disabled={!info}><ChevronRight size={18} /></button>
          </div>
          <input type="range" min={bounds[0]} max={bounds[1]} value={frame} disabled={!info} onChange={event => showFrame(Number(event.target.value))} />
          <span className="frame-counter">{info ? `${frame} / ${bounds[1]}` : "0 / 0"}</span>
          <select value={speed} onChange={event => setSpeed(Number(event.target.value))} disabled={!info}>
            <option value={0.25}>0.25×</option><option value={0.5}>0.5×</option><option value={1}>1×</option><option value={2}>2×</option>
          </select>
        </div>
      </section>

      <aside className="inspector">
        <div className="panel-heading"><span>Inspector</span></div>
        {!info ? <div className="empty-side">Stream properties will appear here.</div> : <>
          <div className="inspect-section">
            <span className="eyebrow">Stream</span>
            <dl><div><dt>Dimensions</dt><dd>{info.width} × {info.height}</dd></div><div><dt>Frames</dt><dd>{info.frameCount}</dd></div><div><dt>Frame rate</dt><dd>{info.fps} fps</dd></div><div><dt>Duration</dt><dd>{(info.frameCount / info.fps).toFixed(2)} s</dd></div><div><dt>Texture</dt><dd>{info.gpuCompressionLabel}</dd></div><div><dt>Compression</dt><dd>LZ4 block</dd></div></dl>
          </div>
          <div className="inspect-section">
            <span className="eyebrow">Selected section</span>
            <h2>{currentRange?.name}</h2>
            <dl><div><dt>Frame range</dt><dd>{bounds[0]}–{bounds[1]}</dd></div><div><dt>End action</dt><dd>{currentRange?.endActionLabel}</dd></div></dl>
            <div className="export-pattern">
              <span className="eyebrow">File name pattern</span>
              <div className="pattern-row">
                <label>
                  <span>Filename</span>
                  <input value={exportName} onChange={event => setExportName(event.target.value)} placeholder={info.fileStem || "export"} />
                </label>
                <label>
                  <span>Separator</span>
                  <input value={exportSeparator} onChange={event => setExportSeparator(event.target.value)} placeholder="_" maxLength={3} />
                </label>
                <label>
                  <span>Start</span>
                  <input type="number" min={0} value={exportStartNumber} onChange={event => setExportStartNumber(event.target.value)} />
                </label>
              </div>
              <small>Preview: {`${exportName || info.fileStem || "export"}${exportSeparator || "_"}${Number.parseInt(exportStartNumber, 10) || 0}.png`}</small>
            </div>
            <button className="button secondary full" onClick={exportFrame}><Download size={15} /> Export current frame</button>
          </div>
          <div className="inspect-section export-section">
            <span className="eyebrow">Export PNG sequences</span>
            <div className="check-list">{info.ranges.map((range, index) => <label key={`${range.name}-${index}`}><input type="checkbox" checked={checked.includes(index)} onChange={() => toggleChecked(index)} /><span className="fake-check">{checked.includes(index) && <Check size={12} />}</span><span>{range.name}</span></label>)}</div>
            <div className="selection-actions"><button onClick={() => setChecked(info.ranges.map((_, index) => index))}>Select all</button><button onClick={() => setChecked([])}>Clear</button></div>
            <button className="button primary full" onClick={exportRanges} disabled={exporting || checked.length === 0}>{exporting ? <RotateCcw className="spin" size={15} /> : <Download size={15} />}{exporting ? "Exporting…" : `Export ${checked.length} section${checked.length === 1 ? "" : "s"}`}</button>
          </div>
        </>}
      </aside>
    </main>
    <footer className="statusbar"><span className={info ? "ready-dot" : "idle-dot"} />{status}<span className="status-right">{info ? `STRM v${info.version} · Format ${info.gpuCompression}` : "Offline"}</span></footer>
  </div>;
}
