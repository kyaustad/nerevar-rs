import { Button } from "@/components/ui/button";
import { useConfig } from "@/features/config/context/config-context-provider";
import { InstanceServerSettingsManager } from "@/features/instances/components/instance-server-settings-manager";
import { ArrowLeft } from "lucide-react";
import { useEffect } from "react";
import { Link, useLocation, useParams } from "wouter";

export function InstanceServerSettingsRoutePage() {
  const params = useParams<{ id: string }>();
  const instanceId = params.id ?? "";
  const config = useConfig();
  const [, navigate] = useLocation();

  const owned = config?.ownedInstances?.find((item) => item.id === instanceId);
  const synced = config?.syncedInstances?.find((item) => item.id === instanceId);
  const instance = owned ?? synced;

  useEffect(() => {
    if (synced && !owned) {
      navigate(`/instances/${encodeURIComponent(instanceId)}`, { replace: true });
    }
  }, [synced, owned, instanceId, navigate]);

  if (!instance) {
    return (
      <div className="mx-auto flex w-full max-w-lg flex-col items-center gap-6 py-12 text-center">
        <p className="font-display text-sm tracking-[0.15em] text-accent uppercase">
          Instance not found
        </p>
        <Button variant="outline" asChild>
          <Link href="/">Back to dashboard</Link>
        </Button>
      </div>
    );
  }

  if (synced && !owned) {
    return null;
  }

  return (
    <div className="mx-auto flex w-full max-w-3xl flex-col gap-4 px-1 pb-8">
      <Button
        variant="outline"
        size="sm"
        className="w-fit font-display text-sm tracking-[0.2em] text-foreground/70 uppercase hover:text-accent"
        asChild
      >
        <Link href={`/instances/${encodeURIComponent(instanceId)}`}>
          <ArrowLeft data-icon="inline-start" />
          Instance detail
        </Link>
      </Button>

      <InstanceServerSettingsManager
        instanceId={instanceId}
        instanceName={instance.name}
      />
    </div>
  );
}
