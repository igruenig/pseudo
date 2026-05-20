import type { ModelDownloadStatus, ModelStatus } from "../core/types";

export function formatModelStatus(status: ModelStatus, download?: ModelDownloadStatus | null): string {
  if (!status.loaded && download?.state === "complete") {
    return `${download.modelName} ready · loads on Analyze`;
  }
  if (!status.loaded) return `${status.modelName ?? download?.modelName ?? "Model"} missing`;
  const backend = status.backend === "metal" ? "Metal" : "CPU";
  const memory = status.residentMemoryMb ? ` · ${status.residentMemoryMb} MB` : "";
  return `${status.modelName ?? "Model"} loaded · ${backend}${memory}`;
}
