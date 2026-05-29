import type { ProcessOutputEvent, ProcessStatusEvent, ProcessStream } from "@/types";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";
import { toast } from "sonner";

export type ProcessLine = {
  id: string;
  stream: ProcessStream;
  text: string;
  at: number;
};

export function useInstanceProcess(
  instanceId: string,
  role: "client" | "server",
  syncedClient = false,
) {
  const [lines, setLines] = useState<ProcessLine[]>([]);
  const [running, setRunning] = useState(false);
  const [launching, setLaunching] = useState(false);

  useEffect(() => {
    void invoke<boolean>("is_instance_process_running", {
      instanceId,
      role,
    }).then(setRunning).catch(() => setRunning(false));
  }, [instanceId, role]);

  useEffect(() => {
    const unlistenOutput = listen<ProcessOutputEvent>("process-output", (event) => {
      if (event.payload.instanceId !== instanceId) return;
      if (event.payload.role !== role) return;
      setLines((prev) => [
        ...prev,
        {
          id: `${prev.length}-${event.payload.line}`,
          stream: event.payload.stream,
          text: event.payload.line,
          at: Date.now(),
        },
      ]);
    });

    const unlistenStatus = listen<ProcessStatusEvent>("process-status", (event) => {
      if (event.payload.instanceId !== instanceId) return;
      if (event.payload.role !== role) return;
      setRunning(event.payload.running);
      if (!event.payload.running && event.payload.exitCode != null) {
        setLines((prev) => [
          ...prev,
          {
            id: `exit-${event.payload.exitCode}`,
            stream: "stderr",
            text: `Process exited with code ${event.payload.exitCode}`,
            at: Date.now(),
          },
        ]);
      }
    });

    return () => {
      void unlistenOutput.then((fn) => fn());
      void unlistenStatus.then((fn) => fn());
    };
  }, [instanceId, role]);

  const launch = useCallback(async () => {
    const command =
      role === "client" ? "launch_instance_client" : "launch_instance_server";
    try {
      setLaunching(true);
      setLines([]);
      await invoke(command, { instanceId });
      setRunning(true);
      toast.success(
        syncedClient && role === "client"
          ? "Up to date — TES3MP client launched"
          : role === "client"
            ? "TES3MP client launched"
            : "TES3MP server launched",
      );
    } catch (error) {
      toast.error(`Launch failed: ${error}`);
      throw error;
    } finally {
      setLaunching(false);
    }
  }, [instanceId, role, syncedClient]);

  const stop = useCallback(async () => {
    try {
      const stopped = await invoke<boolean>("stop_instance_process", {
        instanceId,
        role,
      });
      if (stopped) {
        toast.info("Process stopped");
      }
      setRunning(false);
    } catch (error) {
      toast.error(`Failed to stop process: ${error}`);
    }
  }, [instanceId, role]);

  const clear = useCallback(() => setLines([]), []);

  return { lines, running, launching, launch, stop, clear };
}
