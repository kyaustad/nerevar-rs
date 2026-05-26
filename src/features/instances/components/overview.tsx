import { useConfig } from "@/features/config/context/config-context-provider";
import type { InstanceConfig } from "@/types";

import { useEffect, useState } from "react";
import {
  ConnectionCard,
  InstanceCard,
  NewConnectionCard,
  NewInstanceCard,
} from "./instance-cards";

export function OwnedInstancesOverview() {
  const config = useConfig();
  const [instances, setInstances] = useState<InstanceConfig[]>(
    config?.ownedInstances || [],
  );

  useEffect(() => {
    setInstances(config?.ownedInstances || []);
  }, [config]);

  if (!config) {
    return (
      <div className="flex h-full min-h-full flex-col items-center justify-center">
        <p className="font-display text-xs tracking-[0.25em] text-foreground/60 uppercase animate-pulse">
          Loading...
        </p>
      </div>
    );
  }

  if (instances.length === 0) {
    return (
      <div className="mx-auto flex w-full max-w-lg flex-col items-center justify-center gap-8">
        <p className="text-center font-display text-xs tracking-[0.15em] text-foreground/65 uppercase">
          No owned instances configured
        </p>
        <NewInstanceCard className="min-w-64" />
      </div>
    );
  }

  return (
    <div className="mx-auto flex w-full max-w-3xl flex-col gap-4">
      <div className="grid gap-4 sm:grid-cols-2">
        {instances.map((instance) => (
          <InstanceCard key={instance.id} instance={instance} />
        ))}
        <NewInstanceCard />
      </div>
    </div>
  );
}

export function SyncedInstancesOverview() {
  const config = useConfig();
  const [instances, setInstances] = useState<InstanceConfig[]>(
    config?.syncedInstances || [],
  );

  useEffect(() => {
    setInstances(config?.syncedInstances || []);
  }, [config]);

  if (!config) {
    return (
      <div className="flex h-full min-h-full flex-col items-center justify-center">
        <p className="font-display text-xs tracking-[0.25em] text-foreground/60 uppercase animate-pulse">
          Loading...
        </p>
      </div>
    );
  }

  if (instances.length === 0) {
    return (
      <div className="mx-auto flex w-full max-w-lg flex-col items-center justify-center gap-8">
        <p className="text-center font-display text-xs tracking-[0.15em] text-foreground/65 uppercase">
          {`You haven't connected to any Nerevar servers yet.`}
        </p>
        <NewConnectionCard className="min-w-64" />
      </div>
    );
  }

  return (
    <div className="mx-auto flex w-full max-w-3xl flex-col gap-4">
      <div className="grid gap-4 sm:grid-cols-2">
        {instances.map((instance) => (
          <ConnectionCard key={instance.id} instance={instance} />
        ))}
        <NewConnectionCard />
      </div>
    </div>
  );
}
