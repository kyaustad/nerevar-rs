import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { SyncProgressPanel } from "@/features/instances/components/sync-progress-panel";
import { useInstanceSync } from "@/features/instances/hooks/use-instance-sync";
import {
  newConnectionDefaultValues,
  newConnectionSchema,
  type NewConnectionFormValues,
} from "@/features/instances/schemas/new-connection-schema";
import { useConfig } from "@/features/config/context/config-context-provider";
import { formatByteSize } from "@/lib/format";
import type { NewConnectionConfig, RemoteManifestSummary } from "@/types";
import { standardSchemaResolver } from "@hookform/resolvers/standard-schema";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { ArrowLeft, Loader2, Radio, Sparkles } from "lucide-react";
import { useEffect, useState } from "react";
import { useForm } from "react-hook-form";
import { toast } from "sonner";
import { Link } from "wouter";
import { navigate } from "wouter/use-browser-location";

const SECTION_LABEL =
  "text-lg font-light font-display tracking-[0.08em] text-foreground";

export function NewConnectionPage() {
  const config = useConfig();
  const [testing, setTesting] = useState(false);
  const [creating, setCreating] = useState(false);
  const [preview, setPreview] = useState<RemoteManifestSummary | null>(null);
  const [syncInstanceId, setSyncInstanceId] = useState("");

  const sync = useInstanceSync(syncInstanceId);

  const form = useForm<NewConnectionFormValues>({
    resolver: standardSchemaResolver(newConnectionSchema),
    defaultValues: {
      ...newConnectionDefaultValues,
      remoteSyncPort: config?.syncPort ?? 25567,
    },
    mode: "onSubmit",
  });

  const testConnection = async () => {
    const values = form.getValues();
    const parsed = newConnectionSchema.safeParse(values);
    if (!parsed.success) {
      form.trigger();
      return;
    }

    setTesting(true);
    setPreview(null);
    try {
      await invoke("ping_remote_nerevar_server", {
        remoteHost: parsed.data.remoteHost,
        remoteSyncPort: parsed.data.remoteSyncPort,
      });
      const summary = await invoke<RemoteManifestSummary>(
        "fetch_remote_manifest_summary",
        {
          remoteHost: parsed.data.remoteHost,
          remoteSyncPort: parsed.data.remoteSyncPort,
          syncPassword: parsed.data.syncPassword || null,
        },
      );
      setPreview(summary);
      toast.success(`Connected to ${summary.instanceName}`);
    } catch (error) {
      toast.error(`Connection test failed: ${error}`);
    } finally {
      setTesting(false);
    }
  };

  const onSubmit = form.handleSubmit(async (values) => {
    setCreating(true);
    try {
      const payload: NewConnectionConfig = {
        remoteHost: values.remoteHost,
        remoteSyncPort: values.remoteSyncPort,
        syncPassword: values.syncPassword,
      };

      const instanceId = await invoke<string>("add_synced_connection", {
        newConnection: payload,
      });
      setSyncInstanceId(instanceId);
      toast.success("Connection created — starting initial sync");
      await sync.startSync(instanceId);
      navigate(`/instances/${encodeURIComponent(instanceId)}`);
    } catch (error) {
      if (!String(error).toLowerCase().includes("cancelled")) {
        toast.error(`Failed to create connection: ${error}`);
      }
    } finally {
      setCreating(false);
      setSyncInstanceId("");
    }
  });

  useEffect(() => {
    const unlisten = listen("on_config_added_connection", () => {
      // Config refresh handled by provider
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, []);

  const busy = testing || creating || sync.syncing;

  return (
    <div className="mx-auto flex w-full max-w-2xl flex-col gap-5 px-1 pb-10">
      <Button
        variant="outline"
        size="sm"
        className="w-fit font-display text-xs tracking-[0.2em] text-foreground/70 uppercase hover:text-accent"
        asChild
      >
        <Link href="/synced-instances">
          <ArrowLeft data-icon="inline-start" />
          Synced instances
        </Link>
      </Button>

      <Card className="gap-0 border-border/80 bg-card/70 py-0 ring-1 ring-accent/20">
        <CardContent className="flex flex-col gap-6 py-6">
          <div>
            <h1 className="font-display text-2xl tracking-[0.08em] text-gradient-gold">
              New connection
            </h1>
            <p className="mt-2 font-serif text-sm text-foreground/75">
              Connect to a Nerevar host. Nerevar installs the host&apos;s
              TES3MP-NG release, syncs its mods and settings, and names the
              instance after the host&apos;s, under your Nerevar folder.
            </p>
          </div>

          <form onSubmit={onSubmit} className="flex flex-col gap-5">
            <div className="grid gap-4 sm:grid-cols-2">
              <Field>
                <FieldLabel className={SECTION_LABEL} htmlFor="remoteHost">
                  Nerevar host
                </FieldLabel>
                <Input
                  id="remoteHost"
                  disabled={busy}
                  {...form.register("remoteHost")}
                />
                {form.formState.errors.remoteHost ? (
                  <FieldError errors={[form.formState.errors.remoteHost]} />
                ) : null}
              </Field>
              <Field>
                <FieldLabel className={SECTION_LABEL} htmlFor="remoteSyncPort">
                  Sync port
                </FieldLabel>
                <Input
                  id="remoteSyncPort"
                  type="number"
                  disabled={busy}
                  {...form.register("remoteSyncPort", { valueAsNumber: true })}
                />
                {form.formState.errors.remoteSyncPort ? (
                  <FieldError errors={[form.formState.errors.remoteSyncPort]} />
                ) : null}
              </Field>
            </div>

            <Field>
              <FieldLabel className={SECTION_LABEL} htmlFor="syncPassword">
                Sync password
              </FieldLabel>
              <Input
                id="syncPassword"
                type="password"
                autoComplete="off"
                disabled={busy}
                {...form.register("syncPassword")}
              />
              <p className="font-serif text-sm text-foreground/60">
                Required when the host&apos;s TES3MP server has a password.
              </p>
            </Field>

            {preview ? (
              <div className="rounded-lg border border-accent/30 bg-accent/5 p-4 space-y-2">
                <p className="font-display text-xs tracking-[0.15em] text-accent uppercase">
                  Remote manifest preview
                </p>
                <p className="font-serif text-sm text-foreground/80">
                  {preview.instanceName} — {preview.packageCount} package(s),{" "}
                  {formatByteSize(preview.totalDownloadBytes)} total
                </p>
                {preview.instanceDescription ? (
                  <p className="font-serif text-sm text-foreground/65">
                    {preview.instanceDescription}
                  </p>
                ) : null}
                <p className="font-mono text-xs text-accent/80">
                  {preview.tes3mpReleaseTag
                    ? `TES3MP-NG ${preview.tes3mpReleaseTag}`
                    : "Host is running an older Nerevar: ask them to update"}
                  {" · "}game port {preview.tes3mpServerPort}
                  {preview.passwordRequired ? " · password required" : ""}
                </p>
                <ul className="font-mono text-[0.65rem] text-foreground/60 space-y-1 max-h-24 overflow-y-auto">
                  {preview.packages.map((pkg) => (
                    <li key={pkg.id}>
                      {pkg.name} ({formatByteSize(pkg.totalSizeBytes)})
                    </li>
                  ))}
                </ul>
              </div>
            ) : null}

            {syncInstanceId &&
            (sync.syncing ||
              sync.progress ||
              sync.resumeStatus?.canResume) ? (
              <SyncProgressPanel
                progress={sync.progress}
                syncing={sync.syncing}
                resumeStatus={sync.resumeStatus}
                onCancel={() => void sync.cancelSync()}
              />
            ) : null}

            <div className="flex flex-wrap gap-2">
              <Button
                type="button"
                variant="outline"
                disabled={busy}
                onClick={() => void testConnection()}
              >
                {testing ? (
                  <Loader2 className="animate-spin" data-icon="inline-start" />
                ) : (
                  <Radio data-icon="inline-start" />
                )}
                Test connection
              </Button>
              <Button
                type="submit"
                variant="launch"
                disabled={busy}
                className="flex-1"
              >
                {creating || sync.syncing ? (
                  <Loader2 className="animate-spin" data-icon="inline-start" />
                ) : (
                  <Sparkles data-icon="inline-start" />
                )}
                Create & sync
              </Button>
            </div>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}
