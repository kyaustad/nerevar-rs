import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import { formatByteSize } from "@/lib/format";
import { cn } from "@/lib/utils";
import type { SyncProgressEvent } from "@/types";
import { Loader2, X } from "lucide-react";

const PHASE_LABELS: Record<SyncProgressEvent["phase"], string> = {
  checkingUpdates: "Checking for updates",
  fetchingManifest: "Fetching manifest",
  downloading: "Downloading",
  applyingLoadOrder: "Applying load order",
  validating: "Verifying files",
  writingLaunchCfg: "Writing launch config",
  complete: "Complete",
  cancelled: "Cancelled",
  failed: "Failed",
};

type SyncProgressPanelProps = {
  progress: SyncProgressEvent | null;
  syncing: boolean;
  onCancel?: () => void;
  className?: string;
};

export function SyncProgressPanel({
  progress,
  syncing,
  onCancel,
  className,
}: SyncProgressPanelProps) {
  if (!syncing && !progress) return null;

  const percent =
    progress && progress.bytesTotal > 0
      ? Math.min(100, Math.round((progress.bytesDone / progress.bytesTotal) * 100))
      : progress?.phase === "complete"
        ? 100
        : 0;

  return (
    <div
      className={cn(
        "rounded-lg border border-border/50 bg-background/30 p-4 space-y-3",
        className,
      )}
    >
      <div className="flex items-center justify-between gap-2">
        <div className="flex items-center gap-2">
          {syncing ? <Loader2 className="size-4 animate-spin text-accent" /> : null}
          <span className="font-display text-xs tracking-[0.15em] text-accent uppercase">
            {progress ? PHASE_LABELS[progress.phase] : "Syncing"}
          </span>
        </div>
        {syncing && onCancel ? (
          <Button variant="ghost" size="sm" onClick={onCancel}>
            <X data-icon="inline-start" />
            Cancel
          </Button>
        ) : null}
      </div>

      {progress?.message ? (
        <p className="font-mono text-xs text-foreground/75 truncate">
          {progress.message}
        </p>
      ) : null}

      {progress?.currentFile ? (
        <p className="font-mono text-[0.65rem] text-foreground/55 truncate">
          {progress.currentFile}
        </p>
      ) : null}

      <Progress value={percent} className="h-2" />

      {progress && progress.bytesTotal > 0 ? (
        <p className="text-[0.65rem] text-foreground/55">
          {formatByteSize(progress.bytesDone)} / {formatByteSize(progress.bytesTotal)}
        </p>
      ) : null}
    </div>
  );
}
