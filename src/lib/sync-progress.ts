import type { SyncProgressEvent } from "@/types";

export function syncOverallPercent(progress: SyncProgressEvent): number {
  if (progress.overallPercent > 0 || progress.phase === "complete") {
    return progress.phase === "complete"
      ? 100
      : Math.min(100, progress.overallPercent);
  }

  if (progress.bytesTotal > 0) {
    return Math.min(
      100,
      Math.round((progress.bytesDone / progress.bytesTotal) * 100),
    );
  }

  return 0;
}

export function syncDownloadPercent(progress: SyncProgressEvent): number {
  if (progress.bytesTotal <= 0) return 0;
  return Math.min(
    100,
    Math.round((progress.bytesDone / progress.bytesTotal) * 100),
  );
}
