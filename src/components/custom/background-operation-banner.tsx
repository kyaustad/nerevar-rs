import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import { useBackgroundOperation } from "@/features/instances/context/background-operation-context";
import { cn } from "@/lib/utils";
import type { BackgroundOperationPhase } from "@/types";
import { CheckCircle2, Loader2, X, XCircle } from "lucide-react";

const PHASE_LABELS: Record<BackgroundOperationPhase, string> = {
  scanningPackages: "Scanning folders",
  mergingLoadOrder: "Merging load order",
  savingLoadOrder: "Saving load order",
  parsingCsv: "Parsing CSV",
  applyingLoadOrder: "Applying load order",
  loadingLoadOrder: "Loading load order",
  resolvingLoadOrder: "Resolving plugins",
  updatingServerMetadata: "Updating server metadata",
  hashingPackage: "Processing package",
  hashingFiles: "Hashing files",
  writingManifest: "Writing manifest",
  writingLaunchCfg: "Writing launch cfg",
  complete: "Complete",
};

export function BackgroundOperationBanner() {
  const { activeOperation, dismissOperation } = useBackgroundOperation();

  if (!activeOperation) {
    return null;
  }

  const { status, label, instanceName, detail, progress } = activeOperation;
  const percent =
    progress && progress.total > 0
      ? Math.min(100, Math.round((progress.step / progress.total) * 100))
      : status === "success"
        ? 100
        : undefined;

  const phaseLabel = progress ? PHASE_LABELS[progress.phase] : null;
  const showProgressBar = status === "running" && percent !== undefined;

  return (
    <div
      className={cn(
        "flex shrink-0 flex-col gap-2 border-t px-3 py-2 text-xs backdrop-blur-sm",
        status === "error"
          ? "border-destructive/30 bg-destructive/10"
          : status === "success"
            ? "border-emerald-500/30 bg-emerald-500/10"
            : "border-border/40 bg-card/85",
      )}
    >
      <div className="flex items-center gap-3">
        <StatusIcon status={status} />

        <div className="flex min-w-0 flex-1 flex-col gap-0.5 sm:flex-row sm:items-center sm:gap-2">
          <span className="truncate font-medium text-foreground/85">
            {label}
          </span>
          <span className="hidden text-foreground/30 sm:inline">·</span>
          <span className="truncate text-foreground/65">{instanceName}</span>
          {phaseLabel && status === "running" ? (
            <>
              <span className="hidden text-foreground/30 sm:inline">·</span>
              <span className="truncate text-accent/90">{phaseLabel}</span>
            </>
          ) : null}
        </div>

        {percent !== undefined && status === "running" ? (
          <span className="shrink-0 font-mono text-[0.65rem] text-foreground/55">
            {percent}%
          </span>
        ) : null}

        {status === "error" ? (
          <Button
            variant="outline"
            size="sm"
            className="h-7 shrink-0 px-2"
            onClick={dismissOperation}
          >
            <X className="size-3" data-icon="inline-start" />
            Dismiss
          </Button>
        ) : null}
      </div>

      {detail && status !== "success" ? (
        <p className="truncate pl-6 font-mono text-[0.65rem] text-foreground/70 sm:pl-7">
          {detail}
        </p>
      ) : null}

      {progress?.currentItem && status === "running" ? (
        <p className="truncate pl-6 font-mono text-[0.65rem] text-foreground/55 sm:pl-7">
          {progress.currentItem}
        </p>
      ) : null}

      {showProgressBar ? <Progress value={percent} className="h-1.5" /> : null}
    </div>
  );
}

function StatusIcon({ status }: { status: "running" | "success" | "error" }) {
  if (status === "running") {
    return <Loader2 className="size-3.5 shrink-0 animate-spin text-accent" />;
  }
  if (status === "success") {
    return <CheckCircle2 className="size-3.5 shrink-0 text-emerald-500" />;
  }
  return <XCircle className="size-3.5 shrink-0 text-destructive" />;
}
