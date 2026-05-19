import type { Finding, ReplacementGroup, SensitiveType } from "./types";

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
  const groups = new Map<string, ReplacementGroup>();

  for (const finding of sorted) {
    const normalized = normalizeOriginal(finding.text);
    const key = `${finding.type}:${normalized}`;
    const existing = groups.get(key);
    if (existing) {
      existing.findingIds.push(finding.id);
      continue;
    }

    const next = (counters.get(finding.type) ?? 0) + 1;
    counters.set(finding.type, next);
    groups.set(key, {
      id: `group-${groups.size + 1}`,
      type: finding.type,
      original: finding.text,
      normalizedOriginal: normalized,
      replacement: `[${TYPE_PREFIX[finding.type]}_${next}]`,
      findingIds: [finding.id],
      enabled: true
    });
  }

  return [...groups.values()];
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
        .map((finding) => ({ finding, replacement: group.replacement }))
    )
    .sort((a, b) => b.finding.start - a.finding.start);

  let output = text;
  for (const { finding, replacement } of ranges) {
    output = `${output.slice(0, finding.start)}${replacement}${output.slice(finding.end)}`;
  }
  return output;
}
