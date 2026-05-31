import { Button } from "@/components/ui/button";
import { InstanceDataManager } from "@/features/instances/components/instance-data-manager";
import { useConfig } from "@/features/config/context/config-context-provider";
import { Link, useParams } from "wouter";

export function InstanceDataManagerPage() {
  const params = useParams<{ id: string }>();
  const instanceId = params.id ?? "";
  const config = useConfig();

  const instance =
    config?.ownedInstances?.find((i) => i.id === instanceId) ??
    config?.syncedInstances?.find((i) => i.id === instanceId);

  if (!instance) {
    return (
      <div className="mx-auto flex max-w-lg flex-col items-center gap-4 py-12 text-center">
        <p className="font-display text-sm tracking-[0.15em] text-accent uppercase">
          Instance not found
        </p>
        <Button variant="outline" asChild>
          <Link href="/">Back to dashboard</Link>
        </Button>
      </div>
    );
  }

  return <InstanceDataManager instance={instance} instanceId={instanceId} />;
}
