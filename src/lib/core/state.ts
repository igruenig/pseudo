import type { AnalysisResult, AnalysisStateName, ReplacementGroup } from "./types";

export function deriveState(text: string, result: AnalysisResult | null, busy: boolean, error: string | null): AnalysisStateName {
  if (busy) return "ANALYZING";
  if (error) return "ERROR";
  if (!text.trim()) return "EMPTY";
  if (!result || result.sourceTextHash !== hashText(text)) return "DIRTY_NEEDS_ANALYSIS";
  return "ANALYZED_READY";
}

export async function hashTextAsync(text: string): Promise<string> {
  const data = new TextEncoder().encode(text);
  const digest = await crypto.subtle.digest("SHA-256", data);
  return [...new Uint8Array(digest)].map((byte) => byte.toString(16).padStart(2, "0")).join("");
}

export function hashText(text: string): string {
  let hash = 2166136261;
  for (let index = 0; index < text.length; index += 1) {
    hash ^= text.charCodeAt(index);
    hash = Math.imul(hash, 16777619);
  }
  return (hash >>> 0).toString(16);
}

export function readinessLabel(state: AnalysisStateName, result: AnalysisResult | null, groups: ReplacementGroup[]): string {
  if (state === "EMPTY") return "Paste text to begin";
  if (state === "DIRTY_NEEDS_ANALYSIS") return "Review paused · analyze again";
  if (state === "ANALYZING") return "Analyzing...";
  if (state === "ERROR") return "Analysis failed";
  if (!result) return "Analyze to begin";

  const enabledCount = groups.filter((group) => group.enabled).length;
  const needsReview = result.findings.filter((finding) => finding.needsReview).length;
  if (result.findings.length === 0) return "No findings";
  if (needsReview > 0) {
    return `Manual review recommended · ${needsReview} need review · ${enabledCount} replacements enabled`;
  }
  return `Ready to copy · ${result.findings.length} findings · ${enabledCount} replacements enabled`;
}
