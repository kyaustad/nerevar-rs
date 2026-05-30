import type { BackgroundOperationProgressEvent } from "@/types";
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

export type BackgroundOperationKind =
  | "scanInstanceData"
  | "importMo2Modlist"
  | "saveLoadOrder"
  | "writeLaunchCfg"
  | "hostManifest";

export type BackgroundOperationStatus = "running" | "success" | "error";

export type BackgroundOperationProgress = {
  phase: BackgroundOperationProgressEvent["phase"];
  message: string;
  step: number;
  total: number;
  currentItem?: string;
};

export type BackgroundOperation = {
  id: string;
  instanceId: string;
  instanceName: string;
  kind: BackgroundOperationKind;
  label: string;
  status: BackgroundOperationStatus;
  detail?: string;
  progress?: BackgroundOperationProgress;
  startedAt: number;
};

const OPERATION_LABELS: Record<BackgroundOperationKind, string> = {
  scanInstanceData: "Scanning data directory",
  importMo2Modlist: "Importing MO2 modlist",
  saveLoadOrder: "Saving load order",
  writeLaunchCfg: "Writing launch config",
  hostManifest: "Building sync manifest",
};

type RunOperationOptions<T> = {
  instanceId: string;
  instanceName: string;
  kind: BackgroundOperationKind;
  detail?: string;
  task: (operationId: string) => Promise<T>;
};

type BackgroundOperationContextValue = {
  activeOperation: BackgroundOperation | null;
  isRunning: (kind?: BackgroundOperationKind) => boolean;
  runOperation: <T>(options: RunOperationOptions<T>) => Promise<T>;
  dismissOperation: () => void;
};

const BackgroundOperationContext =
  createContext<BackgroundOperationContextValue | null>(null);

export function BackgroundOperationProvider({
  children,
}: {
  children: ReactNode;
}) {
  const [activeOperation, setActiveOperation] =
    useState<BackgroundOperation | null>(null);

  useEffect(() => {
    const unlisten = listen<BackgroundOperationProgressEvent>(
      "background-operation-progress",
      (event) => {
        const payload = event.payload;
        setActiveOperation((current) => {
          if (!current || current.id !== payload.operationId) {
            return current;
          }
          return {
            ...current,
            detail: payload.message,
            progress: {
              phase: payload.phase,
              message: payload.message,
              step: payload.step,
              total: payload.total,
              currentItem: payload.currentItem ?? undefined,
            },
          };
        });
      },
    );

    return () => {
      void unlisten.then((fn) => fn());
    };
  }, []);

  const dismissOperation = useCallback(() => {
    setActiveOperation(null);
  }, []);

  const runOperation = useCallback(
    async <T,>(options: RunOperationOptions<T>): Promise<T> => {
      const id = crypto.randomUUID();
      setActiveOperation({
        id,
        instanceId: options.instanceId,
        instanceName: options.instanceName,
        kind: options.kind,
        label: OPERATION_LABELS[options.kind],
        status: "running",
        detail: options.detail,
        startedAt: Date.now(),
      });

      try {
        const result = await options.task(id);
        setActiveOperation((current) =>
          current?.id === id
            ? { ...current, status: "success", detail: undefined, progress: undefined }
            : current,
        );
        window.setTimeout(() => {
          setActiveOperation((current) =>
            current?.id === id && current.status === "success" ? null : current,
          );
        }, 3200);
        return result;
      } catch (error) {
        const message = String(error);
        setActiveOperation((current) =>
          current?.id === id
            ? { ...current, status: "error", detail: message, progress: undefined }
            : current,
        );
        throw error;
      }
    },
    [],
  );

  const isRunning = useCallback(
    (kind?: BackgroundOperationKind) => {
      if (!activeOperation || activeOperation.status !== "running") {
        return false;
      }
      if (!kind) return true;
      return activeOperation.kind === kind;
    },
    [activeOperation],
  );

  const value = useMemo(
    () => ({
      activeOperation,
      isRunning,
      runOperation,
      dismissOperation,
    }),
    [activeOperation, dismissOperation, isRunning, runOperation],
  );

  return (
    <BackgroundOperationContext.Provider value={value}>
      {children}
    </BackgroundOperationContext.Provider>
  );
}

export function useBackgroundOperation() {
  const context = useContext(BackgroundOperationContext);
  if (!context) {
    throw new Error(
      "useBackgroundOperation must be used within BackgroundOperationProvider",
    );
  }
  return context;
}
