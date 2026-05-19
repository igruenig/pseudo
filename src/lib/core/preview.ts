import type { AnalysisResult, ReplacementGroup } from "./types";
import { applyReplacements } from "./replacements";

export function buildPreview(text: string, result: AnalysisResult | null, groups: ReplacementGroup[]): string {
  if (!result) return "";
  return applyReplacements(text, result.findings, groups);
}
