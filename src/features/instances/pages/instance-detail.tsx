import { Button } from "@/components/ui/button";
import { useConfig } from "@/features/config/context/config-context-provider";
import { InstanceConfig } from "@/types";
import { invoke } from "@tauri-apps/api/core";
import { Link, useParams } from "wouter";

export function InstanceDetailPage() {
  const params = useParams<{ id: string }>();
  const id = params.id ?? "unknown";

  const config = useConfig();
  const ownedInstances = config?.ownedInstances;
  const syncedInstances = config?.syncedInstances;

  const ownedInstance = ownedInstances?.find((instance) => instance.id === id);
  const syncedInstance = syncedInstances?.find(
    (instance) => instance.id === id,
  );

  const instance = ownedInstance ?? syncedInstance;
  const type: "owned" | "synced" = ownedInstance ? "owned" : "synced";

  if (!instance) {
    return (
      <div className="flex flex-col items-center gap-6 py-8 text-center">
        <p className="font-display text-sm tracking-[0.15em] text-accent uppercase">
          Instance not found
        </p>
      </div>
    );
  }

  if (type === "owned") {
    return <OwnedInstanceDetail instance={instance} />;
  } else {
    return <SyncedInstanceDetail instance={instance} />;
  }
}

function OwnedInstanceDetail({ instance }: { instance: InstanceConfig }) {
  return (
    <div className="flex flex-col items-center gap-6 py-8 text-center">
      <p className="font-display text-sm tracking-[0.15em] text-accent uppercase">
        Owned Instance
      </p>
      <h2 className="font-display text-2xl tracking-[0.08em] text-gradient-gold">
        {instance?.name}
      </h2>
      <p className="max-w-md font-serif text-sm leading-relaxed text-foreground/70">
        {instance.description || "No description"}
      </p>
      <Button
        variant="outline"
        onClick={() => invoke<void>("open_directory", { path: instance.path })}
      >
        Open Instance Directory
      </Button>
      <Button
        variant="outline"
        onClick={() =>
          invoke<void>("open_directory", { path: instance.dataDir })
        }
      >
        Open Data Directory
      </Button>
      <Button variant="outline" asChild>
        <Link href="/">Back to instances</Link>
      </Button>
    </div>
  );
}

function SyncedInstanceDetail({ instance }: { instance: InstanceConfig }) {
  return (
    <div className="flex flex-col items-center gap-6 py-8 text-center">
      <p className="font-display text-sm tracking-[0.15em] text-accent uppercase">
        Synced Instance
      </p>
      <h2 className="font-display text-2xl tracking-[0.08em] text-gradient-gold">
        {instance?.name}
      </h2>
      <p className="max-w-md font-serif text-sm leading-relaxed text-foreground/70">
        {instance.description || "No description"}
      </p>
      <Button variant="outline" asChild>
        <Link href="/">Back to instances</Link>
      </Button>
    </div>
  );
}
