import type {
  GlobalProcessStatus,
  ProcessOutputEvent,
  ProcessStatusEvent,
} from "@/types";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { toast } from "sonner";

export type ProcessLine = {
  id: string;
  stream: "stdout" | "stderr";
  text: string;
  at: number;
};

type RoleState = {
  instanceId: string | null;
  running: boolean;
  launching: boolean;
  lines: ProcessLine[];
};

type ProcessRole = "client" | "server";

type ProcessStatusContextValue = {
  client: RoleState;
  server: RoleState;
  refresh: () => Promise<void>;
  launch: (
    instanceId: string,
    role: ProcessRole,
    syncedClient?: boolean,
  ) => Promise<void>;
  stop: (role: ProcessRole) => Promise<void>;
  clear: (role: ProcessRole) => void;
  canLaunch: (instanceId: string, role: ProcessRole) => boolean;
  resolveInstanceName: (instanceId: string | null) => string | null;
};

const emptyRoleState = (): RoleState => ({
  instanceId: null,
  running: false,
  launching: false,
  lines: [],
});

const ProcessStatusContext = createContext<ProcessStatusContextValue | null>(
  null,
);

function applyGlobalStatus(
  status: GlobalProcessStatus,
  setClient: React.Dispatch<React.SetStateAction<RoleState>>,
  setServer: React.Dispatch<React.SetStateAction<RoleState>>,
) {
  setClient((prev) => ({
    ...prev,
    instanceId: status.clientInstanceId ?? prev.instanceId,
    running: Boolean(status.clientInstanceId),
    launching: status.clientInstanceId ? false : prev.launching,
  }));
  setServer((prev) => ({
    ...prev,
    instanceId: status.serverInstanceId ?? prev.instanceId,
    running: Boolean(status.serverInstanceId),
    launching: status.serverInstanceId ? false : prev.launching,
  }));
}

export function ProcessStatusProvider({
  children,
  resolveInstanceName,
}: {
  children: ReactNode;
  resolveInstanceName: (instanceId: string | null) => string | null;
}) {
  const [client, setClient] = useState<RoleState>(emptyRoleState);
  const [server, setServer] = useState<RoleState>(emptyRoleState);

  const refresh = useCallback(async () => {
    try {
      const status = await invoke<GlobalProcessStatus>("get_global_process_status");
      applyGlobalStatus(status, setClient, setServer);
    } catch {
      setClient((prev) => ({ ...prev, running: false, launching: false }));
      setServer((prev) => ({ ...prev, running: false, launching: false }));
    }
  }, []);

  useEffect(() => {
    void refresh();
    const interval = window.setInterval(() => void refresh(), 5000);

    const unlistenOutput = listen<ProcessOutputEvent>("process-output", (event) => {
      const { instanceId, role, stream, line } = event.payload;
      const appendLine = (prev: RoleState): RoleState => ({
        ...prev,
        instanceId,
        lines: [
          ...prev.lines,
          {
            id: `${prev.lines.length}-${Date.now()}-${line}`,
            stream,
            text: line,
            at: Date.now(),
          },
        ],
      });

      if (role === "client") {
        setClient(appendLine);
      } else if (role === "server") {
        setServer(appendLine);
      }
    });

    const unlistenStatus = listen<ProcessStatusEvent>("process-status", (event) => {
      const { instanceId, role, running, exitCode } = event.payload;
      const applyStatus = (prev: RoleState): RoleState => {
        const next: RoleState = {
          ...prev,
          instanceId: running ? instanceId : prev.instanceId ?? instanceId,
          running,
          launching: false,
        };
        if (!running && exitCode != null) {
          next.lines = [
            ...prev.lines,
            {
              id: `exit-${exitCode}-${Date.now()}`,
              stream: "stderr",
              text: `Process exited with code ${exitCode}`,
              at: Date.now(),
            },
          ];
        }
        return next;
      };

      if (role === "client") {
        setClient(applyStatus);
      } else if (role === "server") {
        setServer(applyStatus);
      }

      void refresh();
    });

    return () => {
      window.clearInterval(interval);
      void unlistenOutput.then((fn) => fn());
      void unlistenStatus.then((fn) => fn());
    };
  }, [refresh]);

  const setRoleState = useCallback(
    (role: ProcessRole, updater: (prev: RoleState) => RoleState) => {
      if (role === "client") {
        setClient(updater);
      } else {
        setServer(updater);
      }
    },
    [],
  );

  const launch = useCallback(
    async (instanceId: string, role: ProcessRole, syncedClient = false) => {
      const command =
        role === "client" ? "launch_instance_client" : "launch_instance_server";

      setRoleState(role, (prev) => ({
        ...prev,
        instanceId,
        launching: true,
        lines: [],
      }));

      try {
        await invoke(command, { instanceId });
        setRoleState(role, (prev) => ({
          ...prev,
          running: true,
          launching: false,
        }));
        toast.success(
          syncedClient && role === "client"
            ? "Up to date — TES3MP client launched"
            : role === "client"
              ? "TES3MP client launched"
              : "TES3MP server launched",
        );
      } catch (error) {
        setRoleState(role, (prev) => ({
          ...prev,
          launching: false,
          running: false,
        }));
        toast.error(`Launch failed: ${error}`);
        throw error;
      } finally {
        setRoleState(role, (prev) => ({ ...prev, launching: false }));
        void refresh();
      }
    },
    [refresh, setRoleState],
  );

  const stop = useCallback(
    async (role: ProcessRole) => {
      const state = role === "client" ? client : server;
      if (!state.instanceId) return;

      try {
        const stopped = await invoke<boolean>("stop_instance_process", {
          instanceId: state.instanceId,
          role,
        });
        if (stopped) {
          toast.info(`${role === "client" ? "Client" : "Server"} stopped`);
        }
        setRoleState(role, (prev) => ({ ...prev, running: false, launching: false }));
      } catch (error) {
        toast.error(`Failed to stop process: ${error}`);
      } finally {
        void refresh();
      }
    },
    [client, refresh, server, setRoleState],
  );

  const clear = useCallback(
    (role: ProcessRole) => {
      setRoleState(role, (prev) => ({ ...prev, lines: [] }));
    },
    [setRoleState],
  );

  const canLaunch = useCallback((_instanceId: string, role: ProcessRole) => {
    const state = role === "client" ? client : server;
    return !state.running && !state.launching;
  }, [client, server]);

  const value = useMemo(
    () => ({
      client,
      server,
      refresh,
      launch,
      stop,
      clear,
      canLaunch,
      resolveInstanceName,
    }),
    [
      canLaunch,
      clear,
      client,
      launch,
      refresh,
      resolveInstanceName,
      server,
      stop,
    ],
  );

  return (
    <ProcessStatusContext.Provider value={value}>
      {children}
    </ProcessStatusContext.Provider>
  );
}

export function useProcessStatus() {
  const ctx = useContext(ProcessStatusContext);
  if (!ctx) {
    throw new Error("useProcessStatus must be used within ProcessStatusProvider");
  }
  return ctx;
}

export function useInstanceProcess(
  instanceId: string,
  role: ProcessRole,
  syncedClient = false,
) {
  const { client, server, launch, stop, clear, canLaunch } = useProcessStatus();
  const roleState = role === "client" ? client : server;

  const launchThis = useCallback(async () => {
    await launch(instanceId, role, syncedClient);
  }, [instanceId, launch, role, syncedClient]);

  return {
    lines: roleState.instanceId === instanceId ? roleState.lines : [],
    running: roleState.instanceId === instanceId && roleState.running,
    launching: roleState.instanceId === instanceId && roleState.launching,
    launch: launchThis,
    stop: () => stop(role),
    clear: () => clear(role),
    canLaunch: canLaunch(instanceId, role),
    busyElsewhere:
      Boolean(roleState.instanceId) &&
      roleState.instanceId !== instanceId &&
      (roleState.running || roleState.launching),
  };
}
