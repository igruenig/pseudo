import { useEffect, useMemo, useRef, useState } from "react";
import { analyzeText, applyReplacementsBackend, cancelModelDownload, copyTextToClipboard, createManualFinding, getModelDownloadStatus, getModelStatus, recomputeAnalysis, startModelDownload } from "./lib/app/tauriApi";
import { formatDownloadStatus } from "./lib/app/modelDownloadStatus";
import { formatModelStatus } from "./lib/app/modelStatus";
import { buildInlineSegments } from "./lib/core/inlineSegments";
import { stringIndexToByteOffset } from "./lib/core/offsets";
import type { AnalysisResult, AnalysisStateName, ModelDownloadStatus, ModelStatus, ReplacementGroup, SensitiveType } from "./lib/core/types";
import "./styles.css";

const MANUAL_TYPES: SensitiveType[] = ["PERSON_NAME", "ORGANIZATION", "LOCATION", "OTHER_SENSITIVE"];
const TYPE_LABELS: Record<SensitiveType, string> = {
  PERSON_NAME: "Person",
  ORGANIZATION: "Organization",
  ROLE_OR_POSITION: "Role",
  LOCATION: "Location",
  EMAIL: "Email",
  PHONE: "Phone",
  DATE: "Date",
  ID_NUMBER: "ID",
  URL: "URL",
  OTHER_SENSITIVE: "Sensitive"
};
type FloatingPoint = { top: number; left: number };
type ManualSelection = FloatingPoint & { start: number; end: number };
const SELECTION_TOOLBAR_WIDTH = 236;
const SELECTION_TOOLBAR_HEIGHT = 44;
const VIEWPORT_MARGIN = 12;

export default function App() {
  const [text, setText] = useState("");
  const [analyzedText, setAnalyzedText] = useState<string | null>(null);
  const [result, setResult] = useState<AnalysisResult | null>(null);
  const [groups, setGroups] = useState<ReplacementGroup[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [modelStatus, setModelStatus] = useState<ModelStatus>({ loaded: false, modelName: "SmolLM3-3B Q4_K_M", backend: "cpu" });
  const [downloadStatus, setDownloadStatus] = useState<ModelDownloadStatus | null>(null);
  const [copied, setCopied] = useState(false);
  const [manualType, setManualType] = useState<SensitiveType>("PERSON_NAME");
  const [manualSelection, setManualSelection] = useState<ManualSelection | null>(null);
  const editorRef = useRef<HTMLTextAreaElement | null>(null);
  const documentRef = useRef<HTMLDivElement | null>(null);
  const toolbarRef = useRef<HTMLElement | null>(null);

  const state: AnalysisStateName = useMemo(() => {
    if (busy) return "ANALYZING";
    if (error) return "ERROR";
    if (!text.trim()) return "EMPTY";
    if (!result || analyzedText !== text) return "DIRTY_NEEDS_ANALYSIS";
    return "ANALYZED_READY";
  }, [analyzedText, busy, error, result, text]);

  const inlineSegments = useMemo(() => buildInlineSegments(text, result, groups), [groups, result, text]);
  const headerStatus = statusLabel(state, result, groups);
  const modelLabel = !downloadStatus || downloadStatus.state === "complete"
    ? formatModelStatus(modelStatus, downloadStatus)
    : formatDownloadStatus(downloadStatus);

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
    setError(null);
    try {
      const confirmed = await applyReplacementsBackend(text, result.sourceTextHash, groups);
      await copyTextToClipboard(confirmed);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1800);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  function handleClear() {
    setText("");
    setAnalyzedText(null);
    setResult(null);
    setGroups([]);
    setError(null);
    setCopied(false);
    setManualSelection(null);
  }

  function handleEditText() {
    setAnalyzedText(null);
    setResult(null);
    setGroups([]);
    setManualSelection(null);
  }

  function toggleGroup(id: string) {
    setGroups((current) => current.map((group) => (group.id === id ? { ...group, enabled: !group.enabled } : group)));
  }

  function handleTextChange(nextText: string) {
    setText(nextText);
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
    const toolbarHalfWidth = SELECTION_TOOLBAR_WIDTH / 2;
    const headerBottom = toolbarRef.current?.getBoundingClientRect().bottom ?? 0;
    const minTop = headerBottom + VIEWPORT_MARGIN;
    const topAbove = rect.top - SELECTION_TOOLBAR_HEIGHT - 8;
    const topBelow = rect.bottom + 8;
    const top = topAbove >= minTop
      ? topAbove
      : Math.min(window.innerHeight - SELECTION_TOOLBAR_HEIGHT - VIEWPORT_MARGIN, Math.max(minTop, topBelow));
    setManualSelection({
      start: Math.min(start, end),
      end: Math.max(start, end),
      top,
      left: Math.min(
        window.innerWidth - toolbarHalfWidth - VIEWPORT_MARGIN,
        Math.max(toolbarHalfWidth + VIEWPORT_MARGIN, rect.left + rect.width / 2)
      )
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

    setError(null);
    try {
      const finding = await createManualFinding(
        crypto.randomUUID(),
        text,
        byteStart,
        byteEnd,
        manualType
      );
      const findings = [...result.findings, finding];
      const { buildGroups } = await import("./lib/core/replacements");
      const nextGroups = buildGroups(findings);
      const nextResult = await recomputeAnalysis(
        crypto.randomUUID(),
        text,
        result.sourceTextHash,
        findings,
        nextGroups
      );
      setResult({ ...nextResult, groups: nextGroups });
      setGroups(nextGroups);
      setManualSelection(null);
      window.getSelection()?.removeAllRanges();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  return (
    <main className="app-shell">
      <header className="toolbar" ref={toolbarRef}>
        <div>
          <h1>Pseudo</h1>
          <p>{headerStatus}</p>
        </div>
        <div className="toolbar-right">
          <div className="model-status">
            <span>{modelLabel}</span>
            {downloadStatus?.state === "downloading" ? (
              <button onClick={handleCancelDownload}>Cancel</button>
            ) : !modelStatus.loaded && downloadStatus?.state !== "complete" ? (
              <button onClick={handleDownloadModel}>Download model</button>
            ) : null}
          </div>
          <div className="toolbar-actions">
            {state === "ANALYZED_READY" ? <button onClick={handleEditText}>Edit text</button> : null}
            <button className={state === "DIRTY_NEEDS_ANALYSIS" ? "primary" : ""} onClick={handleAnalyze} disabled={state !== "DIRTY_NEEDS_ANALYSIS"}>Analyze</button>
            <button onClick={handleClear} disabled={state === "EMPTY"}>Clear</button>
            <button className={state === "ANALYZED_READY" ? "primary" : ""} onClick={handleCopy} disabled={state !== "ANALYZED_READY"}>Copy result</button>
          </div>
        </div>
      </header>

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
                  onClick={() => {
                    toggleGroup(segment.group.id);
                    setManualSelection(null);
                  }}
                  aria-pressed={segment.group.enabled}
                  aria-label={`${segment.group.enabled ? "Keep original" : "Replace"} ${segment.originalText}`}
                  title={`${segment.originalText} · ${segment.finding.type} · click to ${segment.group.enabled ? "keep original" : "replace"}`}
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
        <div
          className="selection-toolbar"
          style={{ top: manualSelection.top, left: manualSelection.left }}
        >
          <select value={manualType} onChange={(event) => setManualType(event.target.value as SensitiveType)}>
            {MANUAL_TYPES.map((type) => <option key={type} value={type}>{TYPE_LABELS[type]}</option>)}
          </select>
          <button onClick={markSelection}>Mark</button>
        </div>
      ) : null}
    </main>
  );
}

function statusLabel(state: AnalysisStateName, result: AnalysisResult | null, groups: ReplacementGroup[]): string {
  if (state === "EMPTY") return "Paste text";
  if (state === "DIRTY_NEEDS_ANALYSIS") return "Needs analysis";
  if (state === "ANALYZING") return "Analyzing";
  if (state === "ERROR") return "Error";
  if (!result || result.findings.length === 0) return "No findings";

  const needsReview = result.findings.filter((finding) => finding.needsReview).length;
  const enabledCount = groups.filter((group) => group.enabled).length;
  if (needsReview > 0) return `Review recommended · ${needsReview}`;
  return `Ready · ${enabledCount} replacements`;
}
