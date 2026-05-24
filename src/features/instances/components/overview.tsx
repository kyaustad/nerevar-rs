import { Button } from "@/components/ui/button";
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { useConfig } from "@/features/config/context/config-context-provider";
import type { InstanceConfig } from "@/types";
import {
  Calendar,
  ChevronRight,
  Layers,
  Play,
  Plus,
  Server,
} from "lucide-react";
import { useEffect, useState } from "react";
import { Link } from "wouter";
import { navigate } from "wouter/use-browser-location";

export function InstancesOverview() {
  const config = useConfig();
  const [instances, setInstances] = useState<InstanceConfig[]>(
    config?.instances || [],
  );

  useEffect(() => {
    setInstances(config?.instances || []);
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
          No instances configured
        </p>
        <NewInstanceCard />
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

function InstanceCard({ instance }: { instance: InstanceConfig }) {
  const href = `/instances/${encodeURIComponent(instance.id)}`;

  return (
    <Link href={href}>
      <Card className="gap-0 border-border/80 bg-card/70 py-0 shadow-[0_0_15px_hsl(var(--accent)/0.08)] ring-accent/20 hover:ring-accent/40 hover:shadow-[0_0_24px_hsl(var(--accent)/0.12)] h-full flex flex-col justify-between">
        <CardHeader className="border-b border-border/50 pb-3 pt-4">
          <CardTitle className="font-display text-sm tracking-[0.15em] text-accent uppercase">
            {instance.name}
          </CardTitle>

          <CardAction>
            <Button
              variant="ghost"
              size="icon-sm"
              className="text-accent/60 hover:text-accent"
            >
              <ChevronRight />
            </Button>
          </CardAction>

          <CardDescription className="font-serif text-[0.95rem] leading-relaxed text-foreground/75">
            {instance.description}
          </CardDescription>
        </CardHeader>

        <CardContent className="flex flex-col gap-4 pb-4 pt-3">
          <div className="flex flex-wrap gap-4 text-xs tracking-wide text-foreground/65">
            <span className="inline-flex items-center gap-1.5">
              <Layers className="size-3.5 text-accent/70" />
              {`##`} mods
            </span>

            <span className="inline-flex items-center gap-1.5">
              <Calendar className="size-3.5 text-accent/70" />

              {`${new Date().toLocaleDateString()} ${new Date().toLocaleTimeString()}`}
            </span>
          </div>

          <div className="grid grid-cols-2 gap-2">
            <Button variant="launch" size="sm" className="h-9 flex-1 text-xs">
              <Play data-icon="inline-start" />
              Launch Client
            </Button>

            <Button variant="server" size="sm" className="h-9 flex-1 text-xs">
              <Server data-icon="inline-start" />
              Launch Server
            </Button>
          </div>
        </CardContent>
      </Card>
    </Link>
  );
}

function NewInstanceCard() {
  return (
    <Link href="/new-instance">
      <Card
        className={
          "flex min-h-[220px] cursor-pointer items-center justify-center bg-transparent py-0 border-3 border-dashed border-accent/30 w-full shadow-none ring-0 hover:border-accent/50 hover:bg-card"
        }
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
