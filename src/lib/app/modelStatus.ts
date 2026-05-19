import type { ModelDownloadStatus, ModelStatus } from "../core/types";

export function formatModelStatus(status: ModelStatus, download?: ModelDownloadStatus | null): string {
  if (!status.loaded && download?.state === "complete") {
    return "Model ready · loads on Analyze";
  }
  if (!status.loaded) return "Model missing";
  const backend = status.backend === "metal" ? "Metal" : "CPU";
  const memory = status.residentMemoryMb ? ` · ${status.residentMemoryMb} MB` : "";
  return `Model loaded · ${backend}${memory}`;
}
