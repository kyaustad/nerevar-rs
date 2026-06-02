import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import { formatByteSize } from "@/lib/format";
import { syncDownloadPercent, syncOverallPercent } from "@/lib/sync-progress";
import { cn } from "@/lib/utils";
import type { InstanceSyncStatus, SyncProgressEvent } from "@/types";
import { Loader2, PauseCircle, X } from "lucide-react";

const PHASE_LABELS: Record<SyncProgressEvent["phase"], string> = {
  checkingUpdates: "Checking for updates",
  verifyingExisting: "Verifying downloaded files",
  fetchingManifest: "Fetching manifest",
  downloading: "Downloading",
  applyingLoadOrder: "Applying load order",
  validating: "Verifying files",
  writingLaunchCfg: "Writing launch config",
  complete: "Complete",
  cancelled: "Paused",
  failed: "Failed",
};

type SyncProgressPanelProps = {
  progress: SyncProgressEvent | null;
  syncing: boolean;
  resumeStatus?: InstanceSyncStatus | null;
  onCancel?: () => void;
  className?: string;
};

export function SyncProgressPanel({
  progress,
  syncing,
  resumeStatus,
  onCancel,
  className,
}: SyncProgressPanelProps) {
  const showResumeBanner =
    !syncing &&
    resumeStatus?.canResume &&
    !resumeStatus.isComplete &&
    resumeStatus.hasManifest;

  const showActivePanel = syncing || progress != null;

  if (!showResumeBanner && !showActivePanel) return null;

  const overallPercent = progress
    ? syncOverallPercent(progress)
    : (resumeStatus?.percentComplete ?? 0);

  const downloadPercent = progress ? syncDownloadPercent(progress) : 0;

  const phaseLabel = progress
    ? PHASE_LABELS[progress.phase]
    : showResumeBanner
      ? "Resume available"
      : "Sync";

  const bytesDone = progress?.bytesDone ?? resumeStatus?.bytesVerified ?? 0;
  const bytesTotal =
    progress?.bytesTotal ?? resumeStatus?.bytesTotal ?? 0;
  const filesDone = progress?.filesDone ?? resumeStatus?.filesVerified ?? 0;
  const filesTotal = progress?.filesTotal ?? resumeStatus?.filesTotal ?? 0;

  const showFileProgress =
    filesTotal > 0 &&
    (progress?.phase === "downloading" ||
      progress?.phase === "verifyingExisting" ||
      showResumeBanner);

  const showDownloadSubBar =
    progress?.phase === "downloading" && bytesTotal > 0;

  return (
    <div className={cn("space-y-3", className)}>
      {showResumeBanner ? (
        <div className="rounded-lg border border-amber-500/40 bg-amber-500/10 p-4 space-y-3">
          <div className="flex items-start gap-3">
            <PauseCircle className="size-5 shrink-0 text-amber-400 mt-0.5" />
            <div className="min-w-0 flex-1 space-y-1">
              <p className="font-display text-xs tracking-[0.15em] text-amber-300 uppercase">
                Sync incomplete — resume available
              </p>
              <p className="text-sm text-foreground/80">
                {resumeStatus.percentComplete}% of the modlist is verified on
                disk. Run <span className="text-foreground">Sync from host</span>{" "}
                to continue where you left off.
              </p>
            </div>
          </div>
          <div className="space-y-1.5">
            <div className="flex items-center justify-between text-[0.65rem] text-foreground/60">
              <span>Overall progress</span>
              <span className="font-mono tabular-nums">
                {resumeStatus.percentComplete}%
              </span>
            </div>
            <Progress
              value={resumeStatus.percentComplete}
              className="h-2.5 bg-background/50"
            />
            <p className="text-[0.65rem] text-foreground/55">
              {formatByteSize(resumeStatus.bytesVerified)} /{" "}
              {formatByteSize(resumeStatus.bytesTotal)}
              {resumeStatus.filesTotal > 0
                ? ` · ${resumeStatus.filesVerified.toLocaleString()} / ${resumeStatus.filesTotal.toLocaleString()} files`
                : null}
            </p>
          </div>
        </div>
      ) : null}

      {showActivePanel ? (
        <div className="rounded-lg border border-border/50 bg-background/30 p-4 space-y-3">
          <div className="flex items-center justify-between gap-2">
            <div className="flex items-center gap-2 min-w-0">
              {syncing ? (
                <Loader2 className="size-4 shrink-0 animate-spin text-accent" />
              ) : null}
              <div className="min-w-0">
                <span className="font-display text-xs tracking-[0.15em] text-accent uppercase block truncate">
                  {phaseLabel}
                </span>
                {syncing && overallPercent > 0 ? (
                  <span className="font-mono text-[0.65rem] text-foreground/50 tabular-nums">
                    Overall {overallPercent}%
                  </span>
                ) : null}
              </div>
            </div>
            {syncing && onCancel ? (
              <Button variant="outline" size="sm" onClick={onCancel}>
                <X data-icon="inline-start" />
                Cancel
              </Button>
            ) : null}
          </div>

          <div className="space-y-1.5">
            <div className="flex items-center justify-between text-[0.65rem] text-foreground/60">
              <span>Overall sync progress</span>
              <span className="font-mono tabular-nums">{overallPercent}%</span>
            </div>
            <Progress
              value={overallPercent}
              className="h-2.5 bg-background/50"
            />
          </div>

          {showDownloadSubBar ? (
            <div className="space-y-1.5 pl-1 border-l border-accent/20">
              <div className="flex items-center justify-between text-[0.65rem] text-foreground/55">
                <span>Downloaded data</span>
                <span className="font-mono tabular-nums">
                  {downloadPercent}%
                </span>
              </div>
              <Progress value={downloadPercent} className="h-1.5" />
            </div>
          ) : null}

          {progress?.message ? (
            <p className="font-mono text-xs text-foreground/75">
              {progress.message}
            </p>
          ) : null}

          {showFileProgress ? (
            <p className="font-mono text-[0.65rem] text-foreground/55">
              Files: {filesDone.toLocaleString()} /{" "}
              {filesTotal.toLocaleString()}
            </p>
          ) : null}

          {progress?.currentFile ? (
            <p className="font-mono text-[0.65rem] text-foreground/45 truncate">
              {progress.currentFile}
            </p>
          ) : null}

          {bytesTotal > 0 ? (
            <p className="text-[0.65rem] text-foreground/55">
              {formatByteSize(bytesDone)} / {formatByteSize(bytesTotal)}
            </p>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}
