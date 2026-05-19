import type { ModelDownloadStatus } from "../core/types";

export function formatDownloadStatus(status: ModelDownloadStatus): string {
  if (status.state === "not_started") return "Model not downloaded";
  if (status.state === "complete") return "Downloaded";
  if (status.state === "cancelled") return "Download cancelled";
  if (status.state === "error") return status.error ?? "Download failed";
  const total = status.totalBytes ? ` / ${formatBytes(status.totalBytes)}` : "";
  return `Downloading ${formatBytes(status.bytesDownloaded)}${total}`;
}

function formatBytes(bytes: number): string {
  if (bytes > 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024 / 1024).toFixed(1)} GB`;
  if (bytes > 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(0)} MB`;
  return `${Math.max(1, Math.round(bytes / 1024))} KB`;
}
