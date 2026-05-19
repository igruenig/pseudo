import type { ModelStatus } from "../core/types";

export function formatModelStatus(status: ModelStatus): string {
  if (!status.loaded) return "Model not loaded";
  const backend = status.backend === "metal" ? "Metal" : "CPU";
  const memory = status.residentMemoryMb ? ` · ${status.residentMemoryMb} MB` : "";
  return `Local model ready · ${backend}${memory}`;
}
