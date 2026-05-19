import type { Finding, ReplacementGroup, SensitiveType } from "./types";
import { byteRangeToStringRange } from "./offsets";

const TYPE_PREFIX: Record<SensitiveType, string> = {
  PERSON_NAME: "PERSON",
  ORGANIZATION: "ORG",
  ROLE_OR_POSITION: "ROLE",
  LOCATION: "LOCATION",
  EMAIL: "EMAIL",
  PHONE: "PHONE",
  DATE: "DATE",
  ID_NUMBER: "ID",
  URL: "URL",
  OTHER_SENSITIVE: "REDACTED"
};

export function normalizeOriginal(text: string): string {
  return text.trim().replace(/\s+/g, " ").toLowerCase();
}

export function buildGroups(findings: Finding[]): ReplacementGroup[] {
  const sorted = [...findings].sort((a, b) => a.start - b.start || a.end - b.end);
  const counters = new Map<SensitiveType, number>();
  const groups: ReplacementGroup[] = [];

  for (const finding of sorted) {
    const normalized = normalizeOriginal(finding.text);
    const existing = groups.find((group) => shouldGroup(group, finding.type, normalized));
    if (existing) {
      existing.findingIds.push(finding.id);
      continue;
    }

    const next = (counters.get(finding.type) ?? 0) + 1;
    counters.set(finding.type, next);
    groups.push({
      id: `group-${groups.length + 1}`,
      type: finding.type,
      original: finding.text,
      normalizedOriginal: normalized,
      replacement: `[${TYPE_PREFIX[finding.type]}_${next}]`,
      findingIds: [finding.id],
      enabled: true
    });
  }

  return groups;
}

export function applyReplacements(
  text: string,
  findings: Finding[],
  groups: ReplacementGroup[]
): string {
  const findingById = new Map(findings.map((finding) => [finding.id, finding]));
  const ranges = groups
    .filter((group) => group.enabled)
    .flatMap((group) =>
      group.findingIds
        .map((id) => findingById.get(id))
        .filter((finding): finding is Finding => Boolean(finding))
        .map((finding) => ({
          ...byteRangeToStringRange(text, finding.start, finding.end),
          replacement: group.replacement
        }))
    )
    .sort((a, b) => b.start - a.start);

  let output = text;
  for (const { start, end, replacement } of ranges) {
    output = `${output.slice(0, start)}${replacement}${output.slice(end)}`;
  }
  return output;
}

function shouldGroup(group: ReplacementGroup, type: SensitiveType, normalized: string): boolean {
  if (group.type !== type) return false;
  if (group.normalizedOriginal === normalized) return true;
  if (type !== "PERSON_NAME") return false;

  const groupAlias = personAliasKey(group.normalizedOriginal);
  const findingAlias = personAliasKey(normalized);
  if (!groupAlias || groupAlias !== findingAlias) return false;

  const groupSimple = stripTrailingSTokens(group.normalizedOriginal);
  const findingSimple = stripTrailingSTokens(normalized);
  return groupSimple.includes(findingSimple) || findingSimple.includes(groupSimple);
}

function personAliasKey(value: string): string {
  return (value.split(/\s+/).at(-1) ?? value).replace(/s$/, "");
}

function stripTrailingSTokens(value: string): string {
  return value.split(/\s+/).map((token) => token.replace(/s$/, "")).join(" ");
}
