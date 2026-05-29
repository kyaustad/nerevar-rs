import type { SyncHostStatus, SyncProgressEvent } from "@/types";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";

export function useSyncHostStatus() {
  const [status, setStatus] = useState<SyncHostStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [activeSync, setActiveSync] = useState<SyncProgressEvent | null>(null);

  const refresh = useCallback(async () => {
    try {
      const next = await invoke<SyncHostStatus>("get_sync_host_status");
      setStatus(next);
    } catch {
      setStatus(null);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
    const interval = window.setInterval(() => void refresh(), 5000);

    const unlistenHosting = listen("hosting-changed", () => void refresh());
    const unlistenSync = listen<SyncProgressEvent>("sync-progress", (event) => {
      const payload = event.payload;
      if (
        payload.phase === "complete" ||
        payload.phase === "cancelled" ||
        payload.phase === "failed"
      ) {
        setActiveSync(null);
      } else {
        setActiveSync(payload);
      }
    });

    return () => {
      window.clearInterval(interval);
      void unlistenHosting.then((fn) => fn());
      void unlistenSync.then((fn) => fn());
    };
  }, [refresh]);

  const stopHosting = useCallback(async () => {
    await invoke("clear_hosting_instance");
    await refresh();
  }, [refresh]);

  return { status, loading, activeSync, refresh, stopHosting };
}
