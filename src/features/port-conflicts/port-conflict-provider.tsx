import { useConfig } from "@/features/config/context/config-context-provider";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Button } from "@/components/ui/button";
import { isTauri } from "@tauri-apps/api/core";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { PortConflict } from "@/types";
import { Loader2, Skull } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { toast } from "sonner";

function conflictKey(conflict: PortConflict) {
  return `${conflict.port}:${conflict.pid}:${conflict.role}`;
}

function mergeConflicts(
  current: PortConflict[],
  incoming: PortConflict[],
): PortConflict[] {
  const map = new Map<string, PortConflict>();
  for (const conflict of [...current, ...incoming]) {
    map.set(conflictKey(conflict), conflict);
  }
  return [...map.values()];
}

function roleLabel(conflict: PortConflict) {
  if (conflict.role === "nerevarSync") {
    return "Nerevar sync server";
  }
  if (conflict.instanceName) {
    return `TES3MP server (${conflict.instanceName})`;
  }
  return "TES3MP server";
}

export function PortConflictProvider({
  children,
}: {
  children: React.ReactNode;
}) {
  const config = useConfig();
  const onboardingComplete = config?.onboardingComplete ?? false;
  const [conflicts, setConflicts] = useState<PortConflict[]>([]);
  const [open, setOpen] = useState(false);
  const [killingPid, setKillingPid] = useState<number | null>(null);

  const refreshConflicts = useCallback(async () => {
    if (!isTauri() || !onboardingComplete) {
      return;
    }
    try {
      const detected = await invoke<PortConflict[]>("check_port_conflicts");
      setConflicts((current) => mergeConflicts(current, detected));
      setOpen(detected.length > 0);
    } catch (error) {
      console.error(error);
    }
  }, [onboardingComplete]);

  useEffect(() => {
    if (!isTauri() || !onboardingComplete) {
      setConflicts([]);
      setOpen(false);
      return;
    }

    void refreshConflicts();

    const unlisten = listen<PortConflict[]>("port-conflicts-detected", (event) => {
      const detected = event.payload ?? [];
      if (detected.length === 0) {
        return;
      }
      setConflicts((current) => mergeConflicts(current, detected));
      setOpen(true);
    });

    return () => {
      void unlisten.then((fn) => fn());
    };
  }, [refreshConflicts, onboardingComplete]);

  const activeConflict = conflicts[0] ?? null;

  const description = useMemo(() => {
    if (!activeConflict) {
      return null;
    }

    return (
      <div className="space-y-3 text-left text-sm leading-relaxed text-muted-foreground">
        <p>
          Nerevar could not use port {activeConflict.port} because another
          process is already listening on it. This often happens after Nerevar
          or TES3MP did not shut down cleanly.
        </p>
        <div className="rounded-md border border-destructive/30 bg-destructive/5 px-3 py-2 space-y-1">
          <p className="font-display text-base text-foreground">
            {roleLabel(activeConflict)} · port {activeConflict.port}
          </p>
          <p className="font-mono text-xs text-foreground/75">
            {activeConflict.processName}
            {activeConflict.pid > 0 ? ` (PID ${activeConflict.pid})` : ""}
          </p>
          {activeConflict.executablePath ? (
            <p className="font-mono text-[0.65rem] text-foreground/60 break-all">
              {activeConflict.executablePath}
            </p>
          ) : null}
        </div>
        {conflicts.length > 1 ? (
          <p>
            {conflicts.length - 1} additional blocked port
            {conflicts.length - 1 === 1 ? "" : "s"} will be shown after this
            one is resolved.
          </p>
        ) : null}
        <p>
          End the blocking process to let Nerevar take over the port. Any
          unsaved work in that process will be lost.
        </p>
      </div>
    );
  }, [activeConflict, conflicts.length]);

  const handleKill = async () => {
    if (!activeConflict || activeConflict.pid <= 0) {
      toast.error("Could not identify which process is using this port.");
      return;
    }

    setKillingPid(activeConflict.pid);
    try {
      await invoke("kill_port_process", { pid: activeConflict.pid });

      const hadSyncConflict = conflicts.some(
        (conflict) => conflict.role === "nerevarSync",
      );

      const remaining = await invoke<PortConflict[]>("check_port_conflicts");
      setConflicts(remaining);

      if (remaining.length === 0) {
        setOpen(false);
        if (hadSyncConflict) {
          await invoke("retry_sync_server");
        }
        toast.success("Port cleared — Nerevar can bind now");
        return;
      }

      if (hadSyncConflict && !remaining.some((c) => c.role === "nerevarSync")) {
        await invoke("retry_sync_server");
      }

      toast.success("Process ended — check remaining port conflicts");
    } catch (error) {
      toast.error(String(error));
    } finally {
      setKillingPid(null);
    }
  };

  return (
    <>
      {children}
      {activeConflict && description ? (
        <AlertDialog
          open={open}
          onOpenChange={(nextOpen) => {
            if (!killingPid) {
              setOpen(nextOpen);
            }
          }}
        >
          <AlertDialogContent className="data-[size=default]:max-w-md data-[size=default]:sm:max-w-lg">
            <AlertDialogHeader className="text-left">
              <AlertDialogTitle className="font-display text-xl tracking-[0.06em]">
                Port already in use
              </AlertDialogTitle>
              <AlertDialogDescription asChild>
                {description}
              </AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter className="flex-col gap-2 sm:flex-row sm:justify-end">
              <AlertDialogCancel disabled={killingPid !== null}>
                Not now
              </AlertDialogCancel>
              <AlertDialogAction
                variant="destructive"
                className="text-primary-foreground"
                disabled={
                  killingPid !== null ||
                  !activeConflict ||
                  activeConflict.pid <= 0
                }
                onClick={(event) => {
                  event.preventDefault();
                  void handleKill();
                }}
              >
                {killingPid !== null ? (
                  <Loader2 className="animate-spin" data-icon="inline-start" />
                ) : (
                  <Skull data-icon="inline-start" />
                )}
                End process and retry
              </AlertDialogAction>
            </AlertDialogFooter>
            {conflicts.length > 1 ? (
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="mx-6 mb-2 w-fit text-xs"
                disabled={killingPid !== null}
                onClick={() => {
                  setConflicts((current) => current.slice(1));
                }}
              >
                Skip this port for now
              </Button>
            ) : null}
          </AlertDialogContent>
        </AlertDialog>
      ) : null}
    </>
  );
}
