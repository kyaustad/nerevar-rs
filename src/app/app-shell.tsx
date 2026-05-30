import { NerevarTitlebarLayout } from "@/components/custom/nerevar-titlebar";
import { useConfig } from "@/features/config/context/config-context-provider";
import { ProcessStatusProvider } from "@/features/instances/context/process-status-context";
import { useCallback, type ReactNode } from "react";

export function AppShell({ children }: { children: ReactNode }) {
  const config = useConfig();

  const resolveInstanceName = useCallback(
    (instanceId: string | null) => {
      if (!instanceId || !config) return null;
      return (
        config.ownedInstances?.find((instance) => instance.id === instanceId)
          ?.name ??
        config.syncedInstances?.find((instance) => instance.id === instanceId)
          ?.name ??
        instanceId
      );
    },
    [config],
  );

  return (
    <ProcessStatusProvider resolveInstanceName={resolveInstanceName}>
      <NerevarTitlebarLayout>{children}</NerevarTitlebarLayout>
    </ProcessStatusProvider>
  );
}
