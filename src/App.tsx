import { useEffect, useMemo, useRef, useState } from "react";
import { analyzeText, applyReplacementsBackend, getModelDownloadStatus, getModelStatus, startModelDownload } from "./lib/app/tauriApi";
import { formatDownloadStatus } from "./lib/app/modelDownloadStatus";
import { formatModelStatus } from "./lib/app/modelStatus";
import { buildPreview } from "./lib/core/preview";
import { hashText, readinessLabel } from "./lib/core/state";
import type { AnalysisResult, AnalysisStateName, ModelDownloadStatus, ModelStatus, ReplacementGroup, SensitiveType } from "./lib/core/types";
import "./styles.css";

const MANUAL_TYPES: SensitiveType[] = ["PERSON_NAME", "ORGANIZATION", "LOCATION", "OTHER_SENSITIVE"];

export default function App() {
  const [text, setText] = useState("");
  const [result, setResult] = useState<AnalysisResult | null>(null);
  const [groups, setGroups] = useState<ReplacementGroup[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [modelStatus, setModelStatus] = useState<ModelStatus>({ loaded: false, backend: "cpu" });
  const [downloadStatus, setDownloadStatus] = useState<ModelDownloadStatus | null>(null);
  const [copied, setCopied] = useState(false);
  const [manualType, setManualType] = useState<SensitiveType>("PERSON_NAME");
  const editorRef = useRef<HTMLTextAreaElement | null>(null);

  const state: AnalysisStateName = useMemo(() => {
    if (busy) return "ANALYZING";
    if (error) return "ERROR";
    if (!text.trim()) return "EMPTY";
    if (!result || result.sourceTextHash !== hashText(text)) return "DIRTY_NEEDS_ANALYSIS";
    return "ANALYZED_READY";
  }, [busy, error, result, text]);

  const preview = useMemo(() => buildPreview(text, result, groups), [groups, result, text]);
  const readiness = readinessLabel(state, result, groups);

  useEffect(() => {
    void refreshModel();
  }, []);

  async function refreshModel() {
    const [status, download] = await Promise.all([getModelStatus(), getModelDownloadStatus()]);
    setModelStatus(status);
    setDownloadStatus(download);
  }

  async function handleAnalyze() {
    const requestId = crypto.randomUUID();
    setBusy(true);
    setError(null);
    try {
      const next = await analyzeText(requestId, text);
      setResult(next);
      setGroups(next.groups);
      await refreshModel();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  async function handleDownloadModel() {
    setError(null);
    try {
      setDownloadStatus(await startModelDownload());
      await refreshModel();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  async function handleCopy() {
    if (!result || state !== "ANALYZED_READY") return;
    const confirmed = await applyReplacementsBackend(text, result.sourceTextHash, groups);
    await navigator.clipboard.writeText(confirmed);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1800);
  }

  function handleClear() {
    setText("");
    setResult(null);
    setGroups([]);
    setError(null);
    setCopied(false);
  }

  function updateGroup(id: string, replacement: string) {
    setGroups((current) => current.map((group) => (group.id === id ? { ...group, replacement } : group)));
  }

  function toggleGroup(id: string) {
    setGroups((current) => current.map((group) => (group.id === id ? { ...group, enabled: !group.enabled } : group)));
  }

  async function markSelection() {
    const start = editorRef.current?.selectionStart ?? -1;
    const end = editorRef.current?.selectionEnd ?? -1;
    if (!result || start < 0 || end <= start) return;
    const selected = text.slice(start, end);
    const finding = {
      id: `manual-${crypto.randomUUID()}`,
      type: manualType,
      start,
      end,
      text: selected,
      source: "MANUAL" as const,
      confidence: 1
    };
    const nextResult = { ...result, findings: [...result.findings, finding] };
    setResult(nextResult);
    setGroups((await import("./lib/core/replacements")).buildGroups(nextResult.findings));
  }

  return (
    <main className="app-shell">
      <header className="toolbar">
        <div>
          <h1>Pseudo</h1>
          <p>{readiness}</p>
        </div>
        <div className="toolbar-actions">
          <button onClick={handleAnalyze} disabled={state !== "DIRTY_NEEDS_ANALYSIS"}>Analyze</button>
          <button onClick={handleClear} disabled={state === "EMPTY"}>Clear</button>
          <button className="primary" onClick={handleCopy} disabled={state !== "ANALYZED_READY"}>Copy result</button>
        </div>
      </header>

      <section className="model-strip">
        <span>{formatModelStatus(modelStatus)}</span>
        <span>{downloadStatus ? formatDownloadStatus(downloadStatus) : "Checking model..."}</span>
        {!modelStatus.loaded && downloadStatus?.state !== "downloading" ? (
          <button onClick={handleDownloadModel}>Download model</button>
        ) : null}
      </section>

      {error ? <div className="error">{error}</div> : null}
      {copied ? <div className="toast">Pseudonymized text copied</div> : null}

      <section className="workspace">
        <section className="editor-pane">
          <div className="pane-heading">Source</div>
          <textarea
            ref={editorRef}
            value={text}
            onChange={(event) => setText(event.target.value)}
            placeholder="Paste text here"
            spellCheck={false}
          />
        </section>

        <section className="replacement-pane">
          <div className="pane-heading">Replacements</div>
          <div className="manual-row">
            <select value={manualType} onChange={(event) => setManualType(event.target.value as SensitiveType)}>
              {MANUAL_TYPES.map((type) => <option key={type}>{type}</option>)}
            </select>
            <button onClick={markSelection} disabled={state !== "ANALYZED_READY"}>Mark sensitive</button>
          </div>
          <div className="replacement-list">
            {groups.map((group) => (
              <label className="replacement-row" key={group.id}>
                <input type="checkbox" checked={group.enabled} onChange={() => toggleGroup(group.id)} />
                <span className="original">{group.original}</span>
                <input value={group.replacement} onChange={(event) => updateGroup(group.id, event.target.value)} />
              </label>
            ))}
            {groups.length === 0 ? <p className="empty-note">No findings yet.</p> : null}
          </div>
        </section>

        <section className="preview-pane">
          <div className="pane-heading">Pseudonymized</div>
          <pre>{preview || "Analyze text to preview the result."}</pre>
        </section>
      </section>
    </main>
  );
}
