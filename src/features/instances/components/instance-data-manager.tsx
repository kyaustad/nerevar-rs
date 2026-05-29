import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { formatByteSize } from "@/lib/format";
import { cn } from "@/lib/utils";
import type {
  InstanceConfig,
  LoadOrder,
  LoadOrderEntry,
  NerevarManifest,
} from "@/types";
import { invoke } from "@tauri-apps/api/core";
import {
  ArrowDown,
  ArrowLeft,
  ArrowUp,
  FolderSync,
  Loader2,
  RefreshCw,
  Save,
} from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { Link } from "wouter";
import { toast } from "sonner";

export const INSTANCE_DATA_CARD_CLASS =
  "gap-0 border-border/80 bg-card/70 py-0 shadow-[0_0_15px_hsl(var(--accent)/0.08)] ring-1 ring-accent/20";

export function sortLoadOrderEntries(entries: LoadOrderEntry[]) {
  return [...entries].sort((a, b) => a.priority - b.priority);
}

type InstanceDataManagerProps = {
  instance: InstanceConfig;
  instanceId: string;
};

export function InstanceDataManager({
  instance,
  instanceId,
}: InstanceDataManagerProps) {
  const [loadOrder, setLoadOrder] = useState<LoadOrder | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [hosting, setHosting] = useState(false);

  const refresh = useCallback(async () => {
    if (!instanceId) return;
    setLoading(true);
    try {
      const order = await invoke<LoadOrder>("scan_instance_data", {
        instanceId,
      });
      setLoadOrder(order);
    } catch (error) {
      toast.error(`Failed to scan data directory: ${error}`);
    } finally {
      setLoading(false);
    }
  }, [instanceId]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const saveLoadOrder = async () => {
    if (!loadOrder) return;
    setSaving(true);
    try {
      await invoke("save_instance_load_order", { instanceId, loadOrder });
      toast.success("Load order saved");
    } catch (error) {
      toast.error(`Failed to save: ${error}`);
    } finally {
      setSaving(false);
    }
  };

  const writeLaunchCfg = async () => {
    try {
      const path = await invoke<string>("write_instance_launch_cfg", {
        instanceId,
      });
      toast.success(`Launch config written to ${path}`);
    } catch (error) {
      toast.error(`Failed to write launch config: ${error}`);
    }
  };

  const startHosting = async () => {
    setHosting(true);
    try {
      await saveLoadOrder();
      const manifest = await invoke<NerevarManifest>("set_hosting_instance", {
        instanceId,
      });
      toast.success(
        `Hosting started (${formatByteSize(manifest.totalDownloadBytes)} in manifest)`,
      );
    } catch (error) {
      toast.error(`Failed to start hosting: ${error}`);
    } finally {
      setHosting(false);
    }
  };

  const moveEntry = (index: number, direction: -1 | 1) => {
    if (!loadOrder) return;
    const sorted = sortLoadOrderEntries(loadOrder.entries);
    const target = index + direction;
    if (target < 0 || target >= sorted.length) return;

    const a = sorted[index];
    const b = sorted[target];
    const aPriority = a.priority;
    a.priority = b.priority;
    b.priority = aPriority;

    setLoadOrder({ ...loadOrder, entries: [...loadOrder.entries] });
  };

  const toggleEntry = (id: string) => {
    if (!loadOrder) return;
    setLoadOrder({
      ...loadOrder,
      entries: loadOrder.entries.map((e) =>
        e.id === id ? { ...e, enabled: !e.enabled } : e,
      ),
    });
  };

  const togglePlugin = (entryId: string, file: string) => {
    if (!loadOrder) return;
    setLoadOrder({
      ...loadOrder,
      entries: loadOrder.entries.map((entry) => {
        if (entry.id !== entryId) return entry;
        return {
          ...entry,
          plugins: entry.plugins.map((p) =>
            p.file === file ? { ...p, enabled: !p.enabled } : p,
          ),
        };
      }),
    });
  };

  const movePlugin = (entryId: string, index: number, direction: -1 | 1) => {
    if (!loadOrder) return;
    setLoadOrder({
      ...loadOrder,
      entries: loadOrder.entries.map((entry) => {
        if (entry.id !== entryId) return entry;
        const plugins = [...entry.plugins];
        const target = index + direction;
        if (target < 0 || target >= plugins.length) return entry;
        [plugins[index], plugins[target]] = [plugins[target], plugins[index]];
        return { ...entry, plugins };
      }),
    });
  };

  const sortedEntries = loadOrder ? sortLoadOrderEntries(loadOrder.entries) : [];

  return (
    <div className="mx-auto flex w-full max-w-3xl flex-col gap-5 px-1 pb-10">
      <Button
        variant="ghost"
        size="sm"
        className="w-fit font-display text-xs tracking-[0.2em] text-foreground/70 uppercase hover:text-accent"
        asChild
      >
        <Link href={`/instances/${encodeURIComponent(instanceId)}`}>
          <ArrowLeft data-icon="inline-start" />
          Instance detail
        </Link>
      </Button>

      <Card className={INSTANCE_DATA_CARD_CLASS}>
        <CardHeader className="border-b border-border/50 space-y-2 pb-4 pt-5">
          <CardTitle className="font-display text-xl tracking-[0.08em] text-gradient-gold">
            Data manager
          </CardTitle>
          <CardDescription className="font-serif text-left text-foreground/75">
            {instance.name} — order mod folders and plugins, then publish a
            manifest for sync.
          </CardDescription>
        </CardHeader>

        <CardContent className="flex flex-col gap-4 py-5">
          <DataManagerToolbar
            loading={loading}
            loadOrder={loadOrder}
            saving={saving}
            hosting={hosting}
            onRefresh={() => void refresh()}
            onSave={() => void saveLoadOrder()}
            onWriteLaunchCfg={() => void writeLaunchCfg()}
            onStartHosting={() => void startHosting()}
          />

          {loadOrder?.baseGameData ? (
            <p className="font-mono text-[0.65rem] text-foreground/60 truncate">
              Base game: {loadOrder.baseGameData}
            </p>
          ) : null}

          <Tabs defaultValue="directories">
            <TabsList variant="line" className="w-full justify-start">
              <TabsTrigger value="directories">Directories</TabsTrigger>
              <TabsTrigger value="plugins">Plugins</TabsTrigger>
            </TabsList>

            <TabsContent value="directories" className="mt-4 space-y-2">
              <DirectoryList
                loading={loading}
                loadOrder={loadOrder}
                sortedEntries={sortedEntries}
                onToggle={toggleEntry}
                onMoveUp={(index) => moveEntry(index, -1)}
                onMoveDown={(index) => moveEntry(index, 1)}
              />
            </TabsContent>

            <TabsContent value="plugins" className="mt-4 space-y-4">
              <PluginList
                sortedEntries={sortedEntries}
                onTogglePlugin={togglePlugin}
                onMovePlugin={movePlugin}
              />
            </TabsContent>
          </Tabs>
        </CardContent>
      </Card>
    </div>
  );
}

function DataManagerToolbar({
  loading,
  loadOrder,
  saving,
  hosting,
  onRefresh,
  onSave,
  onWriteLaunchCfg,
  onStartHosting,
}: {
  loading: boolean;
  loadOrder: LoadOrder | null;
  saving: boolean;
  hosting: boolean;
  onRefresh: () => void;
  onSave: () => void;
  onWriteLaunchCfg: () => void;
  onStartHosting: () => void;
}) {
  return (
    <div className="flex flex-wrap gap-2">
      <Button variant="outline" size="sm" disabled={loading} onClick={onRefresh}>
        {loading ? (
          <Loader2 className="animate-spin" data-icon="inline-start" />
        ) : (
          <RefreshCw data-icon="inline-start" />
        )}
        Rescan
      </Button>
      <Button
        variant="outline"
        size="sm"
        disabled={!loadOrder || saving}
        onClick={onSave}
      >
        <Save data-icon="inline-start" />
        Save order
      </Button>
      <Button
        variant="secondary"
        size="sm"
        disabled={!loadOrder}
        onClick={onWriteLaunchCfg}
      >
        Write launch cfg
      </Button>
      <Button
        variant="launch"
        size="sm"
        disabled={!loadOrder || hosting}
        onClick={onStartHosting}
      >
        {hosting ? (
          <Loader2 className="animate-spin" data-icon="inline-start" />
        ) : (
          <FolderSync data-icon="inline-start" />
        )}
        Save & host manifest
      </Button>
    </div>
  );
}

function DirectoryList({
  loading,
  loadOrder,
  sortedEntries,
  onToggle,
  onMoveUp,
  onMoveDown,
}: {
  loading: boolean;
  loadOrder: LoadOrder | null;
  sortedEntries: LoadOrderEntry[];
  onToggle: (id: string) => void;
  onMoveUp: (index: number) => void;
  onMoveDown: (index: number) => void;
}) {
  if (loading && !loadOrder) {
    return <p className="text-sm text-foreground/60">Scanning…</p>;
  }

  if (loading) {
    return (
      <p className="text-sm text-foreground/60">
        Rescanning…
      </p>
    );
  }

  if (sortedEntries.length === 0) {
    return (
      <p className="font-serif text-sm text-foreground/65">
        No packages found. Add mod folders directly in the instance data
        directory (e.g.{" "}
        <code className="text-accent">Better Bodies/</code> with an .esp, or{" "}
        <code className="text-accent">Rock Replacer/</code> with
        textures/meshes), then rescan.
      </p>
    );
  }

  return sortedEntries.map((entry, index) => (
    <DirectoryRow
      key={entry.id}
      entry={entry}
      index={index}
      total={sortedEntries.length}
      onToggle={() => onToggle(entry.id)}
      onMoveUp={() => onMoveUp(index)}
      onMoveDown={() => onMoveDown(index)}
    />
  ));
}

function PluginList({
  sortedEntries,
  onTogglePlugin,
  onMovePlugin,
}: {
  sortedEntries: LoadOrderEntry[];
  onTogglePlugin: (entryId: string, file: string) => void;
  onMovePlugin: (entryId: string, index: number, direction: -1 | 1) => void;
}) {
  const withPlugins = sortedEntries.filter((e) => e.plugins.length > 0);

  return (
    <>
      {withPlugins.map((entry) => (
        <div
          key={entry.id}
          className="rounded-lg border border-border/50 bg-background/20 p-3"
        >
          <div className="mb-2 flex items-center gap-2">
            <span className="font-display text-xs tracking-[0.15em] text-accent uppercase">
              {entry.name}
            </span>
            {!entry.enabled ? (
              <Badge variant="secondary" className="text-[0.6rem]">
                Dir disabled
              </Badge>
            ) : null}
          </div>
          <ul className="space-y-1">
            {entry.plugins.map((plugin, pluginIndex) => (
              <li
                key={plugin.file}
                className="flex items-center justify-between gap-2 rounded-md px-2 py-1.5 hover:bg-muted/40"
              >
                <label className="flex flex-1 cursor-pointer items-center gap-2 font-mono text-xs">
                  <input
                    type="checkbox"
                    checked={plugin.enabled}
                    disabled={!entry.enabled}
                    onChange={() => onTogglePlugin(entry.id, plugin.file)}
                  />
                  {plugin.file}
                </label>
                <div className="flex gap-1">
                  <Button
                    variant="ghost"
                    size="icon-xs"
                    disabled={pluginIndex === 0}
                    onClick={() => onMovePlugin(entry.id, pluginIndex, -1)}
                  >
                    <ArrowUp />
                  </Button>
                  <Button
                    variant="ghost"
                    size="icon-xs"
                    disabled={pluginIndex === entry.plugins.length - 1}
                    onClick={() => onMovePlugin(entry.id, pluginIndex, 1)}
                  >
                    <ArrowDown />
                  </Button>
                </div>
              </li>
            ))}
          </ul>
        </div>
      ))}
      {withPlugins.length === 0 ? (
        <p className="font-serif text-sm text-foreground/65">
          No plugins detected in scanned folders.
        </p>
      ) : null}
    </>
  );
}

function DirectoryRow({
  entry,
  index,
  total,
  onToggle,
  onMoveUp,
  onMoveDown,
}: {
  entry: LoadOrderEntry;
  index: number;
  total: number;
  onToggle: () => void;
  onMoveUp: () => void;
  onMoveDown: () => void;
}) {
  return (
    <div
      className={cn(
        "flex items-center gap-2 rounded-lg border border-border/50 px-3 py-2",
        !entry.enabled && "opacity-50",
      )}
    >
      <label className="flex min-w-0 flex-1 cursor-pointer items-center gap-2">
        <input type="checkbox" checked={entry.enabled} onChange={onToggle} />
        <div className="min-w-0 flex-1">
          <p className="truncate font-display text-sm text-foreground">
            {entry.name}
          </p>
          <p className="truncate font-mono text-[0.65rem] text-foreground/55">
            {entry.relativeDir}
          </p>
        </div>
      </label>
      <Badge variant="outline" className="shrink-0 text-[0.6rem] uppercase">
        {entry.kind}
      </Badge>
      <div className="flex shrink-0 gap-1">
        <Button
          variant="ghost"
          size="icon-xs"
          disabled={index === 0}
          onClick={onMoveUp}
        >
          <ArrowUp />
        </Button>
        <Button
          variant="ghost"
          size="icon-xs"
          disabled={index === total - 1}
          onClick={onMoveDown}
        >
          <ArrowDown />
        </Button>
      </div>
    </div>
  );
}
