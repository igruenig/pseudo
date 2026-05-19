import { invoke } from "@tauri-apps/api/core";
import { byteRangeToStringRange, stringIndexToByteOffset } from "../core/offsets";
import type { AnalysisResult, Finding, ModelDownloadStatus, ModelStatus, ReplacementGroup, SensitiveType } from "../core/types";

const isTauri = "__TAURI_INTERNALS__" in window;

export async function analyzeText(requestId: string, text: string): Promise<AnalysisResult> {
  if (!isTauri) {
    return mockAnalyze(requestId, text);
  }
  return invoke("analyze_text", { requestId, text });
}

export async function applyReplacementsBackend(
  text: string,
  expectedTextHash: string,
  groups: ReplacementGroup[]
): Promise<string> {
  if (!isTauri) return mockAnalyze("preview", text).then((result) => result.pseudonymizedText);
  return invoke("apply_replacements", { text, expectedTextHash, groups });
}

export async function copyTextToClipboard(text: string): Promise<void> {
  if (!isTauri) {
    await writeBrowserClipboard(text);
    return;
  }
  return invoke("copy_text_to_clipboard", { text });
}

export async function createManualFinding(
  requestId: string,
  text: string,
  start: number,
  end: number,
  type: SensitiveType
): Promise<Finding> {
  if (!isTauri) {
    const range = byteRangeToStringRange(text, start, end);
    return {
      id: `manual-${Date.now()}`,
      type,
      start,
      end,
      text: text.slice(range.start, range.end),
      source: "MANUAL",
      confidence: 1
    };
  }
  return invoke("create_manual_finding", { requestId, text, start, end, type });
}

export async function recomputeAnalysis(
  requestId: string,
  text: string,
  expectedTextHash: string,
  findings: Finding[],
  groups: ReplacementGroup[]
): Promise<AnalysisResult> {
  if (!isTauri) {
    const { buildGroups, applyReplacements } = await import("../core/replacements");
    const nextGroups = buildGroups(findings);
    return {
      requestId,
      sourceTextHash: expectedTextHash,
      findings,
      groups: nextGroups,
      pseudonymizedText: applyReplacements(text, findings, groups.length > 0 ? groups : nextGroups),
      warnings: []
    };
  }
  return invoke("recompute_analysis", { requestId, text, expectedTextHash, findings, groups });
}

export async function getModelStatus(): Promise<ModelStatus> {
  if (!isTauri) return { loaded: false, backend: "cpu" };
  return invoke("get_model_status");
}

export async function getModelDownloadStatus(): Promise<ModelDownloadStatus> {
  if (!isTauri) {
    return {
      state: "not_started",
      modelName: "Qwen3-1.7B Q4_K_M",
      destinationPath: "app data",
      bytesDownloaded: 0
    };
  }
  return invoke("get_model_download_status");
}

export async function startModelDownload(): Promise<ModelDownloadStatus> {
  if (!isTauri) {
    return {
      state: "error",
      modelName: "Qwen3-1.7B Q4_K_M",
      destinationPath: "app data",
      bytesDownloaded: 0,
      error: "Model download is available in the desktop app."
    };
  }
  return invoke("start_model_download");
}

export async function cancelModelDownload(): Promise<ModelDownloadStatus> {
  if (!isTauri) {
    return {
      state: "cancelled",
      modelName: "Qwen3-1.7B Q4_K_M",
      destinationPath: "app data",
      bytesDownloaded: 0
    };
  }
  return invoke("cancel_model_download");
}

async function mockAnalyze(requestId: string, text: string): Promise<AnalysisResult> {
  const { buildGroups, applyReplacements } = await import("../core/replacements");
  const { hashText } = await import("../core/state");
  const findings: Finding[] = [];
  const emailRe = /\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b/gi;
  const urlRe = /\b(?:https?:\/\/|www\.)\S+/gi;
  for (const re of [emailRe, urlRe]) {
    for (const match of text.matchAll(re)) {
      const start = match.index ?? 0;
      const end = start + match[0].length;
      findings.push({
        id: `finding-${findings.length + 1}`,
        type: match[0].includes("@") ? "EMAIL" : "URL",
        start: stringIndexToByteOffset(text, start),
        end: stringIndexToByteOffset(text, end),
        text: match[0],
        source: "DETERMINISTIC",
        confidence: 1
      });
    }
  }
  const groups = buildGroups(findings);
  return {
    requestId,
    sourceTextHash: hashText(text),
    findings,
    groups,
    pseudonymizedText: applyReplacements(text, findings, groups),
    warnings: ["Browser preview uses deterministic email/URL detection only."]
  };
}

async function writeBrowserClipboard(value: string) {
  if (navigator.clipboard?.writeText) {
    try {
      await navigator.clipboard.writeText(value);
      return;
    } catch {
      // Some browser contexts expose navigator.clipboard but reject writes; fall back to selection copy.
    }
  }

  const textarea = document.createElement("textarea");
  textarea.value = value;
  textarea.readOnly = true;
  textarea.style.position = "fixed";
  textarea.style.left = "-9999px";
  textarea.style.top = "0";
  document.body.appendChild(textarea);
  textarea.focus();
  textarea.select();
  const copiedWithFallback = document.execCommand("copy");
  document.body.removeChild(textarea);

  if (!copiedWithFallback) {
    throw new Error("Copy failed. Select the document text and copy manually.");
  }
}
