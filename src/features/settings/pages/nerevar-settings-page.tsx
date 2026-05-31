import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useConfig } from "@/features/config/context/config-context-provider";
import { useSyncHostStatus } from "@/features/instances/hooks/use-sync-host-status";
import { cn } from "@/lib/utils";
import { invoke } from "@tauri-apps/api/core";
import {
  ArrowLeft,
  Loader2,
  Network,
  Save,
  Settings2,
} from "lucide-react";
import { useEffect, useState } from "react";
import { toast } from "sonner";
import { Link } from "wouter";

const cardClass =
  "gap-0 border-border/80 bg-card/70 py-0 shadow-[0_0_15px_hsl(var(--accent)/0.08)] ring-1 ring-accent/20";

export function NerevarSettingsPage() {
  const config = useConfig();
  const { status, refresh: refreshSyncStatus } = useSyncHostStatus();
  const [syncPort, setSyncPort] = useState(config?.syncPort ?? 25567);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (config?.syncPort != null) {
      setSyncPort(config.syncPort);
    }
  }, [config?.syncPort]);

  const hasChanges = config != null && syncPort !== config.syncPort;
  const portValid =
    Number.isInteger(syncPort) && syncPort >= 1 && syncPort <= 65535;

  const saveSettings = async () => {
    if (!portValid) {
      toast.error("Sync port must be a whole number between 1 and 65535");
      return;
    }

    setSaving(true);
    try {
      await invoke("set_sync_port", { port: syncPort });
      await refreshSyncStatus();
      toast.success("Settings saved — sync server restarted on the new port");
    } catch (error) {
      toast.error(String(error));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="mx-auto flex w-full max-w-2xl flex-col gap-5 px-1 pb-10">
      <Button
        variant="outline"
        size="sm"
        className="w-fit font-display text-sm tracking-[0.2em] text-foreground/70 uppercase hover:text-accent"
        asChild
      >
        <Link href="/">
          <ArrowLeft data-icon="inline-start" />
          Dashboard
        </Link>
      </Button>

      <Card className={cardClass}>
        <CardHeader className="border-b border-border/50 space-y-3 pb-4 pt-5">
          <div className="flex items-center gap-2 text-accent">
            <Settings2 className="size-5" />
            <span className="font-display text-[0.7rem] tracking-[0.2em] uppercase">
              Application
            </span>
          </div>
          <CardTitle className="font-display text-3xl tracking-[0.08em] text-gradient-gold">
            Nerevar settings
          </CardTitle>
          <CardDescription className="font-serif text-left text-[1rem] leading-relaxed text-foreground/75">
            Global options that apply to this Nerevar installation.
          </CardDescription>
        </CardHeader>

        <CardContent className="flex flex-col gap-6 py-5">
          <section className="space-y-4">
            <div className="flex flex-wrap items-center gap-2">
              <h2 className="font-display text-lg tracking-[0.15em] text-accent uppercase">
                Sync server
              </h2>
              {status ? (
                <Badge
                  variant={status.serverOnline ? "default" : "secondary"}
                  className="font-mono text-[0.65rem]"
                >
                  {status.serverOnline ? "Online" : "Offline"} · :{status.syncPort}
                </Badge>
              ) : null}
            </div>
            <p className="font-serif text-sm leading-relaxed text-foreground/70">
              The local HTTP port Nerevar listens on for manifest sync, remote
              connections, and instance hosting. Changing it saves to your config
              and restarts the embedded sync server automatically.
            </p>

            <div className="space-y-2">
              <Label
                htmlFor="nerevar-sync-port"
                className="font-display text-[0.75rem] tracking-[0.2em] uppercase text-foreground/70"
              >
                Sync port
              </Label>
              <div className="flex items-center gap-2">
                <Network className="size-4 shrink-0 text-accent/70" />
                <Input
                  id="nerevar-sync-port"
                  type="number"
                  min={1}
                  max={65535}
                  value={syncPort}
                  disabled={saving || !config}
                  onChange={(event) => setSyncPort(Number(event.target.value))}
                  className="max-w-[12rem] font-mono tabular-nums"
                />
              </div>
              {!portValid ? (
                <p className="font-serif text-sm text-destructive">
                  Enter a port between 1 and 65535.
                </p>
              ) : null}
            </div>
          </section>

          {config?.rootPath ? (
            <section className="space-y-2 rounded-lg border border-border/50 bg-background/30 px-3 py-3">
              <h3 className="font-display text-sm tracking-[0.15em] text-foreground/70 uppercase">
                Instance root
              </h3>
              <p className="font-serif text-sm text-foreground/65">
                Default parent folder for new instances. Change this during
                onboarding or when creating instances individually.
              </p>
              <code className="block truncate font-mono text-xs text-foreground/80">
                {config.rootPath}
              </code>
            </section>
          ) : null}

          <Button
            variant="launch"
            className={cn(
              "h-10 w-full font-display text-base tracking-[0.15em] uppercase",
            )}
            disabled={saving || !config || !hasChanges || !portValid}
            onClick={() => void saveSettings()}
          >
            {saving ? (
              <Loader2 className="animate-spin" data-icon="inline-start" />
            ) : (
              <Save data-icon="inline-start" />
            )}
            Save and restart sync server
          </Button>
        </CardContent>
      </Card>
    </div>
  );
}
