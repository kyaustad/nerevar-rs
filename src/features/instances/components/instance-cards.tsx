import { Button } from "@/components/ui/button";
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { cn } from "@/lib/utils";
import { Link } from "wouter";
import {
  Calendar,
  ChevronRight,
  Folder,
  Layers,
  Play,
  Plus,
  Server,
  Settings,
  Sparkles,
  type LucideIcon,
} from "lucide-react";
import { InstanceConfig } from "@/types";
import { useProcessStatus } from "@/features/instances/context/process-status-context";

const dashboardGoldCardClass = cn(
  "dashboard-nav-card relative flex min-h-[220px] w-full cursor-pointer items-center justify-center overflow-hidden border-0 py-0 shadow-none ring-1 ring-accent/25",
);

const dashboardGoldIconClass = cn(
  "dashboard-nav-card-icon flex size-14 items-center justify-center rounded-xl",
);

const dashboardGoldLabelClass = cn(
  "dashboard-nav-card-label dashboard-nav-card-label-accent font-display text-sm font-medium uppercase",
);

function DashboardGoldCard({
  href,
  icon: Icon,
  label,
  className,
}: {
  href: string;
  icon: LucideIcon;
  label: string;
  className?: string;
}) {
  return (
    <Link href={href} className="block h-full w-full">
      <Card className={cn(dashboardGoldCardClass, className)}>
        <CardContent className="relative z-10 flex flex-col items-center gap-4 py-10">
          <div className={dashboardGoldIconClass}>
            <Icon className="size-6 stroke-[2.5]" />
          </div>
          <span className={cn(dashboardGoldLabelClass, "text-lg ")}>
            {label}
          </span>
        </CardContent>
      </Card>
    </Link>
  );
}

function InstanceCardMeta({ instance }: { instance: InstanceConfig }) {
  const syncedLabel = instance.lastSyncedAt
    ? new Date(instance.lastSyncedAt).toLocaleString()
    : `${new Date().toLocaleDateString()} ${new Date().toLocaleTimeString()}`;

  return (
    <div className="flex flex-wrap gap-4 text-sm tracking-wide text-foreground/65">
      <span className="inline-flex items-center gap-1.5">
        <Layers className="size-3.5 text-accent/70" />
        {instance.remoteHost ? "Synced" : "Local"}
      </span>
      <span className="inline-flex items-center gap-1.5">
        <Calendar className="size-3.5 text-accent/70" />
        {syncedLabel}
      </span>
    </div>
  );
}

function InstanceLaunchButtons({
  instanceId,
  clientOnly = false,
}: {
  instanceId: string;
  clientOnly?: boolean;
}) {
  const { launch, canLaunch, client, server } = useProcessStatus();
  const clientDisabled = !canLaunch(instanceId, "client");
  const serverDisabled = !canLaunch(instanceId, "server");
  const clientBusyElsewhere =
    client.instanceId !== null &&
    client.instanceId !== instanceId &&
    (client.running || client.launching);
  const serverBusyElsewhere =
    server.instanceId !== null &&
    server.instanceId !== instanceId &&
    (server.running || server.launching);

  if (clientOnly) {
    return (
      <Button
        variant="launch"
        className="h-10 w-full text-sm"
        disabled={clientDisabled || clientBusyElsewhere}
        onClick={() => void launch(instanceId, "client", true)}
      >
        <Play data-icon="inline-start" />
        Launch Client
      </Button>
    );
  }

  return (
    <div className="grid grid-cols-2 gap-2">
      <Button
        variant="launch"
        className="h-10 flex-1 text-xs"
        disabled={clientDisabled || clientBusyElsewhere}
        onClick={() => void launch(instanceId, "client")}
      >
        <Play data-icon="inline-start" />
        Launch Client
      </Button>
      <Button
        variant="server"
        className="h-10 flex-1 text-xs"
        disabled={serverDisabled || serverBusyElsewhere}
        onClick={() => void launch(instanceId, "server")}
      >
        <Server data-icon="inline-start" />
        Launch Server
      </Button>
    </div>
  );
}

function InstanceOverviewCard({
  instance,
  clientOnly = false,
}: {
  instance: InstanceConfig;
  clientOnly?: boolean;
}) {
  const href = `/instances/${encodeURIComponent(instance.id)}`;

  return (
    <Card className="gap-0 border-border/80 bg-card/70 py-0 shadow-[0_0_15px_hsl(var(--accent)/0.08)] ring-accent/20 hover:ring-accent/40 hover:shadow-[0_0_24px_hsl(var(--accent)/0.12)] h-full flex flex-col justify-between">
      <Link href={href} className="block">
        <CardHeader className="border-b border-border/50 pb-3 pt-4">
          <CardTitle className="font-display text-base tracking-[0.15em] text-accent uppercase">
            {instance.name}
          </CardTitle>
          <CardAction>
            <Button
              variant="outline"
              size="icon-sm"
              className="text-accent/60 hover:text-accent"
              tabIndex={-1}
            >
              <ChevronRight />
            </Button>
          </CardAction>
          <CardDescription className="font-serif text-base leading-relaxed text-foreground/75 line-clamp-2 ">
            {instance.description}
          </CardDescription>
        </CardHeader>
      </Link>
      <CardContent className="flex flex-col gap-4 pb-4 pt-3">
        <InstanceCardMeta instance={instance} />
        <InstanceLaunchButtons
          instanceId={instance.id}
          clientOnly={clientOnly}
        />
      </CardContent>
    </Card>
  );
}

export function ConnectionCard({ instance }: { instance: InstanceConfig }) {
  return <InstanceOverviewCard instance={instance} clientOnly />;
}

export function InstanceCard({ instance }: { instance: InstanceConfig }) {
  return <InstanceOverviewCard instance={instance} />;
}

export function NewConnectionCard({ className }: { className?: string }) {
  return (
    <Link href="/new-connection">
      <Card
        className={cn(
          "flex min-h-[220px] cursor-pointer items-center justify-center bg-transparent py-0 border-3 border-dashed border-accent/30 w-full shadow-none ring-0 hover:border-accent/50 hover:bg-card",
          className,
        )}
      >
        <CardContent className="flex flex-col items-center gap-3 py-8">
          <div className="flex size-12 items-center justify-center rounded-lg border border-accent/40 text-accent/70">
            <Play className="size-6" />
          </div>

          <span className="font-display text-xs tracking-[0.25em] text-foreground/60 uppercase">
            New Connection
          </span>
        </CardContent>
      </Card>
    </Link>
  );
}
export function NewInstanceCard({ className }: { className?: string }) {
  return (
    <Link href="/new-instance">
      <Card
        className={cn(
          "flex min-h-[220px] cursor-pointer items-center justify-center bg-transparent py-0 border-3 border-dashed border-accent/30 w-full shadow-none ring-0 hover:border-accent/50 hover:bg-card",
          className,
        )}
      >
        <CardContent className="flex flex-col items-center gap-3 py-8">
          <div className="flex size-12 items-center justify-center rounded-lg border border-accent/40 text-accent/70">
            <Plus className="size-6" />
          </div>

          <span className="font-display text-xs tracking-[0.25em] text-foreground/60 uppercase">
            New Instance
          </span>
        </CardContent>
      </Card>
    </Link>
  );
}

export function DashboardOwnedInstancesCard() {
  return (
    <DashboardGoldCard
      href="/owned-instances"
      icon={Server}
      label="Owned Instances"
    />
  );
}

export function DashboardUpdateAvailableCard({
  version,
}: {
  version?: string;
}) {
  return (
    <Link href="/update-available" className="block h-full w-full my-4">
      <Card
        className={cn(
          " relative w-full cursor-pointer overflow-hidden py-0 shadow-none ring-1 ring-accent",
          "max-h-18 min-h-18 flex flex-row items-center justify-center animate-pulse border-accent border-2",
        )}
      >
        <CardContent className="relative z-10 flex flex-row items-center gap-4 py-10 ">
          <div
            className={
              " flex size-10 items-center justify-center rounded-xl border-2 border-accent"
            }
          >
            <Sparkles className="size-5 stroke-[2.5] text-emerald-500" />
          </div>
          <span className={cn(dashboardGoldLabelClass, "text-lg font-bold ")}>
            {version ? `Update to v${version}` : "Update Available!"}
          </span>
        </CardContent>
      </Card>
    </Link>
  );
}
export function DashboardSyncedInstancesCard() {
  return (
    <DashboardGoldCard
      href="/synced-instances"
      icon={Play}
      label="Synced Instances"
    />
  );
}
export function DashboardSettingsCard({ className }: { className?: string }) {
  return (
    <DashboardGoldCard
      href="/settings"
      icon={Settings}
      label="Nerevar Settings"
      className={className}
    />
  );
}
export function DashboardMO2Card({ className }: { className?: string }) {
  return (
    <DashboardGoldCard
      href="/mo2"
      icon={Folder}
      label="MO2 Plugin"
      className={className}
    />
  );
}
