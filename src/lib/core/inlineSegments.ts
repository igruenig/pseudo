import { byteRangeToStringRange } from "./offsets";
import type { AnalysisResult, Finding, ReplacementGroup } from "./types";

export type InlineSegment =
  | {
      kind: "text";
      key: string;
      text: string;
      byteStart: number;
      byteEnd: number;
    }
  | {
      kind: "entity";
      key: string;
      finding: Finding;
      group: ReplacementGroup;
      originalText: string;
      displayText: string;
      stringStart: number;
      stringEnd: number;
    };

export function buildInlineSegments(text: string, result: AnalysisResult | null, groups: ReplacementGroup[]): InlineSegment[] {
  if (!result) {
    return [{ kind: "text", key: "empty", text, byteStart: 0, byteEnd: new TextEncoder().encode(text).length }];
  }

  const groupByFindingId = new Map<string, ReplacementGroup>();
  for (const group of groups) {
    for (const findingId of group.findingIds) {
      groupByFindingId.set(findingId, group);
    }
  }

  const findings = result.findings
    .map((finding) => ({ finding, group: groupByFindingId.get(finding.id) }))
    .filter((item): item is { finding: Finding; group: ReplacementGroup } => Boolean(item.group))
    .sort((a, b) => a.finding.start - b.finding.start || a.finding.end - b.finding.end);

  const segments: InlineSegment[] = [];
  let stringCursor = 0;
  let byteCursor = 0;

  for (const { finding, group } of findings) {
    const range = byteRangeToStringRange(text, finding.start, finding.end);
    if (range.start < stringCursor) continue;

    if (range.start > stringCursor) {
      segments.push({
        kind: "text",
        key: `text-${segments.length}`,
        text: text.slice(stringCursor, range.start),
        byteStart: byteCursor,
        byteEnd: finding.start
      });
    }

    segments.push({
      kind: "entity",
      key: finding.id,
      finding,
      group,
      originalText: text.slice(range.start, range.end),
      displayText: group.enabled ? group.replacement : text.slice(range.start, range.end),
      stringStart: range.start,
      stringEnd: range.end
    });

    stringCursor = range.end;
    byteCursor = finding.end;
  }

  if (stringCursor < text.length) {
    segments.push({
      kind: "text",
      key: `text-${segments.length}`,
      text: text.slice(stringCursor),
      byteStart: byteCursor,
      byteEnd: new TextEncoder().encode(text).length
    });
  }

  return segments;
}
