// abstract the config to be a context provider for the app so child components can consume and access the config and tie a tauri
// event "on_config_change" to update the config in the context

import { createContext, useContext, useEffect, useState } from "react";
import type { NerevarConfig } from "@/types";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";

const ConfigContext = createContext<NerevarConfig | null>(null);

export function ConfigContextProvider({
  children,
}: {
  children: React.ReactNode;
}) {
  const [config, setConfig] = useState<NerevarConfig | null>(null);
  useEffect(() => {
    const handleGetNerevarConfig = async () => {
      try {
        const config = await invoke<NerevarConfig>(
          "load_or_create_nerevar_config",
        );
        setConfig(config);
      } catch (error) {
        console.error(error);
      }
    };
    handleGetNerevarConfig();
    const unlisten = listen("on_config_change", (event) => {
      try {
        const config = event.payload as NerevarConfig;
        setConfig(config);
      } catch (error) {
        console.error(error);
      }
    });
    return () => {
      unlisten.then((unlistenFn) => unlistenFn());
    };
  }, []);
  return (
    <ConfigContext.Provider value={config}>{children}</ConfigContext.Provider>
  );
}

export function useConfig() {
  return useContext(ConfigContext);
}
