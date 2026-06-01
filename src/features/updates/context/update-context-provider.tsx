import { createContext, useContext, useEffect, useState } from "react";
import { useConfig } from "@/features/config/context/config-context-provider";
import type { AppUpdateStatus } from "@/types";
import { invoke } from "@tauri-apps/api/core";

type UpdateContextValue = {
  status: AppUpdateStatus | null;
  loading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
};

const UpdateContext = createContext<UpdateContextValue | null>(null);

export function UpdateContextProvider({
  children,
}: {
  children: React.ReactNode;
}) {
  const config = useConfig();
  const [status, setStatus] = useState<AppUpdateStatus | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = async () => {
    setLoading(true);
    setError(null);
    try {
      const next = await invoke<AppUpdateStatus>("check_for_app_update");
      setStatus(next);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    if (!config?.onboardingComplete) {
      return;
    }
    void refresh();
  }, [config?.onboardingComplete]);

  return (
    <UpdateContext.Provider value={{ status, loading, error, refresh }}>
      {children}
    </UpdateContext.Provider>
  );
}

export function useAppUpdate() {
  const ctx = useContext(UpdateContext);
  if (!ctx) {
    throw new Error("useAppUpdate must be used within UpdateContextProvider");
  }
  return ctx;
}
