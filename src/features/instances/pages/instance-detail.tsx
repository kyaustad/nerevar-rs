import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import { ProcessConsole } from "@/features/instances/components/process-console";
import { SyncProgressPanel } from "@/features/instances/components/sync-progress-panel";
import { useConfig } from "@/features/config/context/config-context-provider";
import { useInstanceProcess } from "@/features/instances/hooks/use-instance-process";
import { useInstanceSync } from "@/features/instances/hooks/use-instance-sync";
import { cn } from "@/lib/utils";
import { InstanceConfig } from "@/types";
import { invoke } from "@tauri-apps/api/core";
import {
  ArrowLeft,
  FolderOpen,
  Network,
  Play,
  RefreshCw,
  Server,
  Settings2,
  Square,
} from "lucide-react";
import type { ReactNode } from "react";
import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import type { SyncProgressEvent } from "@/types";
import { toast } from "sonner";
import { Link, useParams } from "wouter";

const detailCardClass =
  "gap-0 border-border/80 bg-card/70 py-0 shadow-[0_0_15px_hsl(var(--accent)/0.08)] ring-1 ring-accent/20";

async function activateNerevarSync(instanceId: string) {
  try {
    await invoke("activate_hosting_instance", { instanceId });
    toast.success("Hosting active — serving your last saved manifest");
  } catch (error) {
    toast.error(String(error));
  }
}

export function InstanceDetailPage() {
  const params = useParams<{ id: string }>();
  const id = params.id ?? "unknown";

  const config = useConfig();
  const ownedInstance = config?.ownedInstances?.find((instance) => instance.id === id);
  const syncedInstance = config?.syncedInstances?.find(
    (instance) => instance.id === id,
  );

  const instance = ownedInstance ?? syncedInstance;
  const type: "owned" | "synced" = ownedInstance ? "owned" : "synced";

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

  if (type === "owned") {
    return <OwnedInstanceDetail instance={instance} />;
  }

  return <SyncedInstanceDetail instance={instance} />;
}

function InstanceDetailShell({
  backHref,
  backLabel,
  kindLabel,
  instance,
  children,
}: {
  backHref: string;
  backLabel: string;
  kindLabel: string;
  instance: InstanceConfig;
  children: ReactNode;
}) {
  const lastSynced = instance.lastSyncedAt
    ? new Date(instance.lastSyncedAt).toLocaleString()
    : null;

  return (
    <div className="mx-auto flex w-full max-w-2xl flex-col gap-5 px-1 pb-8">
      <Button
        variant="ghost"
        size="sm"
        className="w-fit font-display text-sm tracking-[0.2em] text-foreground/70 uppercase hover:text-accent"
        asChild
      >
        <Link href={backHref}>
          <ArrowLeft data-icon="inline-start" />
          {backLabel}
        </Link>
      </Button>

      <Card className={detailCardClass}>
        <CardHeader className="border-b border-border/50 space-y-3 pb-4 pt-5">
          <div className="flex flex-wrap items-center gap-2">
            <Badge
              variant="outline"
              className="border-accent/40 font-display text-[0.7rem] tracking-[0.15em] text-accent uppercase"
            >
              {kindLabel}
            </Badge>
            {lastSynced ? (
              <Badge variant="secondary" className="font-mono text-[0.65rem]">
                Synced {lastSynced}
              </Badge>
            ) : null}
          </div>
          <CardTitle className="font-display text-3xl tracking-[0.08em] text-gradient-gold">
            {instance.name}
          </CardTitle>
          <CardDescription className="font-serif text-left text-[1rem] leading-relaxed text-foreground/75">
            {instance.description || "No description provided."}
          </CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-5 py-5">{children}</CardContent>
      </Card>
    </div>
  );
}

function DetailSection({
  title,
  description,
  children,
}: {
  title: string;
  description?: string;
  children: ReactNode;
}) {
  return (
    <section className="flex flex-col gap-3">
      <div>
        <h3 className="font-display text-lg tracking-[0.15em] text-accent uppercase">
          {title}
        </h3>
        {description ? (
          <p className="mt-1 font-serif text-base leading-relaxed tracking-[0.03em] text-foreground/65">
            {description}
          </p>
        ) : null}
      </div>
      {children}
    </section>
  );
}

function PathRow({ label, path }: { label: string; path: string }) {
  return (
    <div className="flex flex-col gap-1 rounded-lg border border-border/50 bg-background/30 px-3 py-2 text-left">
      <span className="font-display text-[0.7rem] tracking-[0.15em] text-foreground/55 uppercase">
        {label}
      </span>
      <code className="truncate font-mono text-[0.8rem] text-foreground/80">{path}</code>
    </div>
  );
}

function ClientPlayControls({
  instanceId,
  description,
  synced = false,
}: {
  instanceId: string;
  description: string;
  synced?: boolean;
}) {
  const client = useInstanceProcess(instanceId, "client", synced);
  const [launchProgress, setLaunchProgress] = useState<SyncProgressEvent | null>(
    null,
  );

  useEffect(() => {
    if (!synced) return;
    const unlisten = listen<SyncProgressEvent>("sync-progress", (event) => {
      if (event.payload.instanceId !== instanceId) return;
      setLaunchProgress(event.payload);
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, [instanceId, synced]);

  return (
    <DetailSection title="Play" description={description}>
      {synced && (client.launching || launchProgress) ? (
        <SyncProgressPanel
          progress={launchProgress}
          syncing={client.launching}
        />
      ) : null}
      <Button
        variant="launch"
        className="h-10 w-full text-base"
        disabled={client.running || client.launching}
        onClick={() => void client.launch()}
      >
        <Play data-icon="inline-start" />
        {client.launching ? "Checking updates…" : "Launch client"}
      </Button>
      <Button
        variant="outline"
        size="sm"
        className="w-full"
        disabled={!client.running}
        onClick={() => void client.stop()}
      >
        <Square data-icon="inline-start" />
        Stop client
      </Button>
      <ProcessConsole
        title="Client output"
        lines={client.lines}
        running={client.running}
        onClear={client.clear}
      />
    </DetailSection>
  );
}

function OwnedPlaySection({ instance }: { instance: InstanceConfig }) {
  const client = useInstanceProcess(instance.id, "client");
  const server = useInstanceProcess(instance.id, "server");

  return (
    <>
      <DetailSection
        title="Play"
        description="Launch TES3MP using the generated OpenMW launch config for this instance."
      >
        <div className="grid gap-2 sm:grid-cols-2">
          <Button
            variant="launch"
            className="h-10 w-full text-base"
            disabled={client.running}
            onClick={() => void client.launch()}
          >
            <Play data-icon="inline-start" />
            Launch client
          </Button>
          <Button
            variant="server"
            className="h-10 w-full text-base"
            disabled={server.running}
            onClick={() => void server.launch()}
          >
            <Server data-icon="inline-start" />
            Launch server
          </Button>
        </div>
        <div className="grid gap-2 sm:grid-cols-2">
          <Button
            variant="outline"
            size="sm"
            disabled={!client.running}
            onClick={() => void client.stop()}
          >
            <Square data-icon="inline-start" />
            Stop client
          </Button>
          <Button
            variant="outline"
            size="sm"
            disabled={!server.running}
            onClick={() => void server.stop()}
          >
            <Square data-icon="inline-start" />
            Stop server
          </Button>
        </div>
      </DetailSection>

      <ProcessConsole
        title="Client output"
        lines={client.lines}
        running={client.running}
        onClear={client.clear}
      />
      <ProcessConsole
        title="Server output"
        lines={server.lines}
        running={server.running}
        onClear={server.clear}
      />
    </>
  );
}

function OwnedInstanceDetail({ instance }: { instance: InstanceConfig }) {
  return (
    <InstanceDetailShell
      backHref="/owned-instances"
      backLabel="Owned instances"
      kindLabel="Owned"
      instance={instance}
    >
      <OwnedPlaySection instance={instance} />

      <Separator className="bg-border/60" />

      <DetailSection
        title="Nerevar sync"
        description="Share mods and settings with connected players. Hosting uses the manifest already saved from the data manager — it does not rescan or rebuild files."
      >
        <Button
          variant="outline"
          className={cn(
            "h-10 w-full font-display text-base tracking-[0.15em] uppercase",
            "border-accent/40 hover:border-accent/60 hover:bg-accent/5",
          )}
          onClick={() => void activateNerevarSync(instance.id)}
        >
          <Network data-icon="inline-start" />
          Start hosting last manifest
        </Button>
        <p className="font-serif text-sm leading-relaxed text-foreground/60">
          To regenerate the manifest after changing mods, open the data manager and use
          &quot;Save &amp; host manifest&quot;.
        </p>
        <Button variant="secondary" className="h-10 w-full" asChild>
          <Link href={`/instances/${encodeURIComponent(instance.id)}/data`}>
            <Settings2 data-icon="inline-start" />
            Open data manager
          </Link>
        </Button>
      </DetailSection>

      <Separator className="bg-border/60" />

      <DetailSection title="Directories" description="Open instance folders on disk.">
        <div className="grid gap-2 sm:grid-cols-2">
          <Button
            variant="outline"
            className="h-10 w-full font-display text-base tracking-[0.15em] uppercase"
            onClick={() => invoke<void>("open_directory", { path: instance.path })}
          >
            <FolderOpen data-icon="inline-start" />
            Instance folder
          </Button>
          <Button
            variant="outline"
            className="h-10 w-full font-display text-base tracking-[0.15em] uppercase"
            onClick={() => invoke<void>("open_directory", { path: instance.dataDir })}
          >
            <FolderOpen data-icon="inline-start" />
            Data folder
          </Button>
        </div>
        <div className="flex flex-col gap-2">
          <PathRow label="Instance path" path={instance.path} />
          <PathRow label="Data path" path={instance.dataDir} />
        </div>
      </DetailSection>
    </InstanceDetailShell>
  );
}

function SyncedInstanceDetail({ instance }: { instance: InstanceConfig }) {
  const sync = useInstanceSync(instance.id);

  return (
    <InstanceDetailShell
      backHref="/synced-instances"
      backLabel="Synced instances"
      kindLabel="Synced"
      instance={instance}
    >
      <DetailSection
        title="Connection"
        description={
          instance.remoteHost
            ? `Nerevar sync at ${instance.remoteHost}:${instance.remoteSyncPort ?? "?"} · TES3MP game port ${instance.tes3mpServerPort ?? "?"}`
            : "Manage your link to this Nerevar server."
        }
      >
        <div className="grid gap-2 sm:grid-cols-2">
          <Button
            variant="outline"
            className={cn(
              "h-10 w-full font-display text-base tracking-[0.15em] uppercase",
              "border-accent/40 hover:border-accent/60 hover:bg-accent/5",
            )}
            disabled={sync.syncing}
            onClick={() => void sync.startSync()}
          >
            {sync.syncing ? (
              <RefreshCw className="animate-spin" data-icon="inline-start" />
            ) : (
              <Network data-icon="inline-start" />
            )}
            Sync from host
          </Button>
          <Button
            variant="secondary"
            className="h-10 w-full"
            disabled={sync.syncing}
            onClick={() => void sync.cancelSync()}
          >
            Cancel sync
          </Button>
        </div>
        <SyncProgressPanel
          progress={sync.progress}
          syncing={sync.syncing}
          onCancel={() => void sync.cancelSync()}
        />
      </DetailSection>

      <Separator className="bg-border/60" />

      <ClientPlayControls
        instanceId={instance.id}
        synced
        description="Checks the host for mod updates, syncs if needed, then connects to the TES3MP server."
      />

      <Separator className="bg-border/60" />

      <DetailSection title="Directories" description="Open instance folders on disk.">
        <div className="grid gap-2 sm:grid-cols-2">
          <Button
            variant="outline"
            className="h-10 w-full"
            onClick={() => invoke<void>("open_directory", { path: instance.path })}
          >
            <FolderOpen data-icon="inline-start" />
            Instance folder
          </Button>
          <Button
            variant="outline"
            className="h-10 w-full"
            onClick={() => invoke<void>("open_directory", { path: instance.dataDir })}
          >
            <FolderOpen data-icon="inline-start" />
            Data folder
          </Button>
        </div>
        <PathRow label="Instance path" path={instance.path} />
        <PathRow label="Data path" path={instance.dataDir} />
      </DetailSection>
    </InstanceDetailShell>
  );
}
