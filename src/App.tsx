import { useEffect, useMemo, useRef, useState } from "react";
import { analyzeText, applyReplacementsBackend, cancelModelDownload, createManualFinding, getModelDownloadStatus, getModelStatus, startModelDownload } from "./lib/app/tauriApi";
import { formatDownloadStatus } from "./lib/app/modelDownloadStatus";
import { formatModelStatus } from "./lib/app/modelStatus";
import { buildInlineSegments } from "./lib/core/inlineSegments";
import { stringIndexToByteOffset } from "./lib/core/offsets";
import { readinessLabel } from "./lib/core/state";
import type { AnalysisResult, AnalysisStateName, ModelDownloadStatus, ModelStatus, ReplacementGroup, SensitiveType } from "./lib/core/types";
import "./styles.css";

const MANUAL_TYPES: SensitiveType[] = ["PERSON_NAME", "ORGANIZATION", "LOCATION", "OTHER_SENSITIVE"];
type FloatingPoint = { top: number; left: number };
type ManualSelection = FloatingPoint & { start: number; end: number };

export default function App() {
  const [text, setText] = useState("");
  const [analyzedText, setAnalyzedText] = useState<string | null>(null);
  const [result, setResult] = useState<AnalysisResult | null>(null);
  const [groups, setGroups] = useState<ReplacementGroup[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [modelStatus, setModelStatus] = useState<ModelStatus>({ loaded: false, backend: "cpu" });
  const [downloadStatus, setDownloadStatus] = useState<ModelDownloadStatus | null>(null);
  const [copied, setCopied] = useState(false);
  const [manualType, setManualType] = useState<SensitiveType>("PERSON_NAME");
  const [selectedGroupId, setSelectedGroupId] = useState<string | null>(null);
  const [popoverPoint, setPopoverPoint] = useState<FloatingPoint | null>(null);
  const [manualSelection, setManualSelection] = useState<ManualSelection | null>(null);
  const editorRef = useRef<HTMLTextAreaElement | null>(null);
  const documentRef = useRef<HTMLDivElement | null>(null);

  const state: AnalysisStateName = useMemo(() => {
    if (busy) return "ANALYZING";
    if (error) return "ERROR";
    if (!text.trim()) return "EMPTY";
    if (!result || analyzedText !== text) return "DIRTY_NEEDS_ANALYSIS";
    return "ANALYZED_READY";
  }, [analyzedText, busy, error, result, text]);

  const inlineSegments = useMemo(() => buildInlineSegments(text, result, groups), [groups, result, text]);
  const readiness = readinessLabel(state, result, groups);
  const selectedGroup = useMemo(
    () => groups.find((group) => group.id === selectedGroupId) ?? null,
    [groups, selectedGroupId]
  );
  const showModelStrip = !downloadStatus || downloadStatus.state !== "complete";

  useEffect(() => {
    void refreshModel();
  }, []);

  useEffect(() => {
    if (downloadStatus?.state !== "downloading") return;
    const interval = window.setInterval(() => {
      void refreshModel();
    }, 750);
    return () => window.clearInterval(interval);
  }, [downloadStatus?.state]);

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
      setAnalyzedText(text);
      setSelectedGroupId(null);
      setManualSelection(null);
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
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  async function handleCancelDownload() {
    setError(null);
    try {
      setDownloadStatus(await cancelModelDownload());
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
    setAnalyzedText(null);
    setResult(null);
    setGroups([]);
    setError(null);
    setCopied(false);
    setSelectedGroupId(null);
    setManualSelection(null);
  }

  function handleEditText() {
    setAnalyzedText(null);
    setResult(null);
    setGroups([]);
    setSelectedGroupId(null);
    setManualSelection(null);
  }

  function updateGroup(id: string, replacement: string) {
    setGroups((current) => current.map((group) => (group.id === id ? { ...group, replacement } : group)));
  }

  function toggleGroup(id: string) {
    setGroups((current) => current.map((group) => (group.id === id ? { ...group, enabled: !group.enabled } : group)));
  }

  function selectGroup(id: string, point: FloatingPoint) {
    setSelectedGroupId(id);
    setPopoverPoint(point);
    setManualSelection(null);
  }

  function handleTextChange(nextText: string) {
    setText(nextText);
    setSelectedGroupId(null);
    setManualSelection(null);
  }

  function firstTextNode(node: Node): Text | null {
    if (node.nodeType === Node.TEXT_NODE) return node as Text;
    for (const child of Array.from(node.childNodes)) {
      const found = firstTextNode(child);
      if (found) return found;
    }
    return null;
  }

  function lastTextNode(node: Node): Text | null {
    if (node.nodeType === Node.TEXT_NODE) return node as Text;
    const children = Array.from(node.childNodes);
    for (let index = children.length - 1; index >= 0; index -= 1) {
      const found = lastTextNode(children[index]);
      if (found) return found;
    }
    return null;
  }

  function byteOffsetFromTextNode(node: Text, offset: number): number | null {
    if (!node.parentElement) return null;
    const segment = node.parentElement.closest<HTMLElement>("[data-byte-start]");
    if (!segment) return null;
    const base = Number(segment.dataset.byteStart);
    if (segment.dataset.entity === "true") {
      return offset <= 0 ? base : Number(segment.dataset.byteEnd);
    }
    const content = node.textContent;
    return base + stringIndexToByteOffset(content, offset);
  }

  function byteOffsetFromSelectionBoundary(node: Node, offset: number, edge: "start" | "end"): number | null {
    if (node.nodeType === Node.TEXT_NODE) {
      return byteOffsetFromTextNode(node as Text, offset);
    }

    const children = Array.from(node.childNodes);
    const candidate = edge === "start"
      ? children.slice(offset).map(firstTextNode).find(Boolean)
      : children.slice(0, offset).reverse().map(lastTextNode).find(Boolean);

    if (!candidate) {
      const element = node instanceof HTMLElement ? node.closest<HTMLElement>("[data-byte-start]") : null;
      if (!element) return null;
      return edge === "start" ? Number(element.dataset.byteStart) : Number(element.dataset.byteEnd);
    }

    return byteOffsetFromTextNode(candidate, edge === "start" ? 0 : candidate.textContent.length);
  }

  function updateReviewSelection() {
    if (state !== "ANALYZED_READY" || !documentRef.current) return;

    const selection = window.getSelection();
    if (!selection || selection.isCollapsed || selection.rangeCount === 0) {
      setManualSelection(null);
      return;
    }

    const range = selection.getRangeAt(0);
    if (!documentRef.current.contains(range.commonAncestorContainer)) {
      setManualSelection(null);
      return;
    }

    const start = byteOffsetFromSelectionBoundary(range.startContainer, range.startOffset, "start");
    const end = byteOffsetFromSelectionBoundary(range.endContainer, range.endOffset, "end");
    if (start === null || end === null || start === end) {
      setManualSelection(null);
      return;
    }

    const rect = range.getBoundingClientRect();
    setSelectedGroupId(null);
    setManualSelection({
      start: Math.min(start, end),
      end: Math.max(start, end),
      top: Math.max(12, rect.top - 46),
      left: rect.left + rect.width / 2
    });
  }

  function handleReviewSelection() {
    window.setTimeout(updateReviewSelection, 0);
  }

  async function markSelection() {
    if (!result) return;
    const selectedStart = manualSelection?.start ?? -1;
    const selectedEnd = manualSelection?.end ?? -1;
    const start = editorRef.current?.selectionStart ?? -1;
    const end = editorRef.current?.selectionEnd ?? -1;
    const byteStart = selectedStart >= 0 ? selectedStart : stringIndexToByteOffset(text, start);
    const byteEnd = selectedEnd >= 0 ? selectedEnd : stringIndexToByteOffset(text, end);
    if (byteStart < 0 || byteEnd <= byteStart) return;

    const finding = await createManualFinding(
      crypto.randomUUID(),
      text,
      byteStart,
      byteEnd,
      manualType
    );
    const nextResult = { ...result, findings: [...result.findings, finding] };
    setResult(nextResult);
    setGroups((await import("./lib/core/replacements")).buildGroups(nextResult.findings));
    setManualSelection(null);
    window.getSelection()?.removeAllRanges();
  }

  return (
    <main className="app-shell">
      <header className="toolbar">
        <div>
          <h1>Pseudo</h1>
          <p>{readiness}</p>
        </div>
        <div className="toolbar-actions">
          {state === "ANALYZED_READY" ? <button onClick={handleEditText}>Edit text</button> : null}
          <button onClick={handleAnalyze} disabled={state !== "DIRTY_NEEDS_ANALYSIS"}>Analyze</button>
          <button onClick={handleClear} disabled={state === "EMPTY"}>Clear</button>
          <button className="primary" onClick={handleCopy} disabled={state !== "ANALYZED_READY"}>Copy result</button>
        </div>
      </header>

      {showModelStrip ? (
        <section className="model-strip">
          <span>{formatModelStatus(modelStatus, downloadStatus)}</span>
          <span>{downloadStatus ? formatDownloadStatus(downloadStatus) : "Checking model..."}</span>
          {downloadStatus?.state === "downloading" ? (
            <button onClick={handleCancelDownload}>Cancel</button>
          ) : !modelStatus.loaded && downloadStatus?.state !== "complete" ? (
            <button onClick={handleDownloadModel}>Download model</button>
          ) : null}
        </section>
      ) : null}

      {error ? <div className="error">{error}</div> : null}
      {copied ? <div className="toast">Pseudonymized text copied</div> : null}

      <section className="document-shell">
        {state === "ANALYZED_READY" && result ? (
          <div
            ref={documentRef}
            className="review-surface"
            onMouseUp={handleReviewSelection}
            onKeyUp={handleReviewSelection}
          >
            {inlineSegments.map((segment) => {
              if (segment.kind === "text") {
                return (
                  <span
                    data-byte-start={segment.byteStart}
                    data-byte-end={segment.byteEnd}
                    key={segment.key}
                  >
                    {segment.text}
                  </span>
                );
              }

              return (
                <button
                  className={[
                    "entity-chip",
                    segment.group.enabled ? "enabled" : "disabled",
                    segment.finding.needsReview ? "needs-review" : ""
                  ].join(" ")}
                  data-byte-end={segment.byteEnd}
                  data-byte-start={segment.byteStart}
                  data-entity="true"
                  key={segment.key}
                  onClick={(event) => {
                    const rect = event.currentTarget.getBoundingClientRect();
                    selectGroup(segment.group.id, {
                      top: rect.bottom + 8,
                      left: rect.left + rect.width / 2
                    });
                  }}
                  title={`${segment.originalText} · ${segment.finding.type}`}
                  type="button"
                >
                  {segment.displayText}
                </button>
              );
            })}
          </div>
        ) : (
          <textarea
            ref={editorRef}
            value={text}
            onChange={(event) => handleTextChange(event.target.value)}
            placeholder="Paste text here"
            spellCheck={false}
            disabled={busy}
          />
        )}
      </section>

      {manualSelection ? (
        <div className="selection-toolbar" style={{ top: manualSelection.top, left: manualSelection.left }}>
          <select value={manualType} onChange={(event) => setManualType(event.target.value as SensitiveType)}>
            {MANUAL_TYPES.map((type) => <option key={type}>{type}</option>)}
          </select>
          <button onClick={markSelection}>Mark</button>
        </div>
      ) : null}

      {selectedGroup && popoverPoint ? (
        <div className="entity-popover" style={{ top: popoverPoint.top, left: popoverPoint.left }}>
          <div className="popover-label">{selectedGroup.type}</div>
          <div className="popover-original">{selectedGroup.original}</div>
          <input
            autoFocus
            value={selectedGroup.replacement}
            onChange={(event) => updateGroup(selectedGroup.id, event.target.value)}
          />
          <label className="popover-toggle">
            <input
              type="checkbox"
              checked={selectedGroup.enabled}
              onChange={() => toggleGroup(selectedGroup.id)}
            />
            Replace this group
          </label>
        </div>
      ) : null}
    </main>
  );
}
