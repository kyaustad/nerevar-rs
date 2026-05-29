import { Button } from "@/components/ui/button";
import { useSyncHostStatus } from "@/features/instances/hooks/use-sync-host-status";
import { cn } from "@/lib/utils";
import { Link } from "wouter";
import { Loader2, Radio, Server, Square } from "lucide-react";
import { toast } from "sonner";

export function SyncHostStatusBar() {
  const { status, loading, activeSync, stopHosting } = useSyncHostStatus();

  if (loading && !status) {
    return (
      <div className="flex h-9 shrink-0 items-center gap-2 border-b border-border/40 bg-background/60 px-3 text-xs text-foreground/50">
        <Loader2 className="size-3 animate-spin" />
        Checking sync server…
      </div>
    );
  }

  if (!status) return null;

  const hosting = Boolean(status.hostingInstanceId);
  const serverOnline = status.serverOnline;

  async function handleStopHosting() {
    try {
      await stopHosting();
      toast.info("Stopped hosting sync");
    } catch (error) {
      toast.error(`Failed to stop hosting: ${error}`);
    }
  }

  return (
    <div className="flex h-9 shrink-0 items-center gap-3 border-b border-border/40 bg-background/60 px-3 text-xs">
      <StatusDot
        online={serverOnline}
        label={serverOnline ? `Sync server online · :${status.syncPort}` : `Sync server offline · :${status.syncPort}`}
      />

      <span className="hidden text-foreground/30 sm:inline">|</span>

      {hosting ? (
        <span className="flex min-w-0 items-center gap-1.5 text-foreground/75">
          <Radio className="size-3 shrink-0 text-emerald-500" />
          <span className="truncate">
            Hosting{" "}
            <strong className="font-medium text-foreground">
              {status.hostingInstanceName ?? status.hostingInstanceId}
            </strong>
            {!status.manifestAvailable ? " (manifest missing)" : ""}
          </span>
        </span>
      ) : (
        <span className="flex items-center gap-1.5 text-foreground/55">
          <Server className="size-3 shrink-0" />
          Not hosting — activate on an owned instance
        </span>
      )}

      {activeSync ? (
        <>
          <span className="hidden text-foreground/30 sm:inline">|</span>
          <span className="flex min-w-0 items-center gap-1.5 text-accent">
            <Loader2 className="size-3 shrink-0 animate-spin" />
            <span className="truncate">
              Syncing {activeSync.instanceId}
              {activeSync.phase ? ` · ${activeSync.phase}` : ""}
            </span>
          </span>
        </>
      ) : null}

      <div className="ml-auto flex shrink-0 items-center gap-1">
        {hosting && status.hostingInstanceId ? (
          <>
            <Button variant="ghost" size="xs" className="h-7 px-2" asChild>
              <Link href={`/instances/${encodeURIComponent(status.hostingInstanceId)}`}>
                View
              </Link>
            </Button>
            <Button
              variant="ghost"
              size="xs"
              className="h-7 px-2 text-foreground/70 hover:text-destructive"
              onClick={() => void handleStopHosting()}
            >
              <Square className="size-3" data-icon="inline-start" />
              Stop
            </Button>
          </>
        ) : null}
      </div>
    </div>
  );
}

function StatusDot({ online, label }: { online: boolean; label: string }) {
  return (
    <span className="flex items-center gap-1.5 text-foreground/70">
      <span
        className={cn(
          "size-2 rounded-full",
          online ? "bg-emerald-500" : "bg-destructive/80",
        )}
      />
      <span>{label}</span>
    </span>
  );
}
