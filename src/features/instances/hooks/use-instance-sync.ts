import type { ManifestValidationResult, SyncProgressEvent } from "@/types";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";
import { toast } from "sonner";

export function useInstanceSync(instanceId: string) {
  const [progress, setProgress] = useState<SyncProgressEvent | null>(null);
  const [syncing, setSyncing] = useState(false);
  const [lastValidation, setLastValidation] =
    useState<ManifestValidationResult | null>(null);

  useEffect(() => {
    const unlisten = listen<SyncProgressEvent>("sync-progress", (event) => {
      if (event.payload.instanceId !== instanceId) return;
      setProgress(event.payload);
      if (
        event.payload.phase === "complete" ||
        event.payload.phase === "cancelled" ||
        event.payload.phase === "failed"
      ) {
        setSyncing(false);
      }
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, [instanceId]);

  const startSync = useCallback(async (overrideInstanceId?: string) => {
    const targetId = overrideInstanceId ?? instanceId;
    if (!targetId) {
      throw new Error("No instance id");
    }
    setSyncing(true);
    setLastValidation(null);
    setProgress(null);
    try {
      const validation = await invoke<ManifestValidationResult>(
        "sync_instance_from_remote",
        { instanceId: targetId },
      );
      setLastValidation(validation);
      if (validation.valid) {
        toast.success("Sync complete — files verified");
      } else {
        toast.error(
          `Sync finished with ${validation.issues.length} validation issue(s)`,
        );
      }
      return validation;
    } catch (error) {
      const message = String(error);
      if (!message.toLowerCase().includes("cancelled")) {
        toast.error(`Sync failed: ${error}`);
      }
      throw error;
    } finally {
      setSyncing(false);
    }
  }, [instanceId]);

  const cancelSync = useCallback(async () => {
    try {
      await invoke<boolean>("cancel_instance_sync", { instanceId });
      toast.info("Sync cancellation requested");
    } catch (error) {
      toast.error(`Failed to cancel sync: ${error}`);
    }
  }, [instanceId]);

  return {
    progress,
    syncing,
    lastValidation,
    startSync,
    cancelSync,
  };
}
