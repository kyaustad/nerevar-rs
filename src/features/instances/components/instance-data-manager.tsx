import { Badge } from "@/components/ui/badge";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import {
  DndContext,
  DragOverlay,
  KeyboardSensor,
  PointerSensor,
  closestCenter,
  type DragEndEvent,
  type DragStartEvent,
  useSensor,
  useSensors,
} from "@dnd-kit/core";
import {
  SortableContext,
  sortableKeyboardCoordinates,
  useSortable,
  verticalListSortingStrategy,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  moveEntryByOffset,
  normalizePriorities,
  reorderLoadOrderEntries,
  setEntryPriority,
  sortLoadOrderEntries,
} from "@/features/instances/lib/load-order-utils";
import { formatByteSize } from "@/lib/format";
import { cn } from "@/lib/utils";
import type {
  InstanceConfig,
  LoadOrder,
  LoadOrderEntry,
  Mo2ModlistImportResult,
  NerevarManifest,
} from "@/types";
import { invoke } from "@tauri-apps/api/core";
import {
  ArrowDown,
  ArrowLeft,
  ArrowUp,
  FolderSync,
  GripVertical,
  Loader2,
  RefreshCw,
  Save,
  Trash2,
  Upload,
} from "lucide-react";
import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type HTMLAttributes,
} from "react";
import { Link } from "wouter";
import { toast } from "sonner";

export const INSTANCE_DATA_CARD_CLASS =
  "gap-0 border-border/80 bg-card/70 py-0 shadow-[0_0_15px_hsl(var(--accent)/0.08)] ring-1 ring-accent/20";

export { sortLoadOrderEntries } from "@/features/instances/lib/load-order-utils";

type InstanceDataManagerProps = {
  instance: InstanceConfig;
  instanceId: string;
};

export function InstanceDataManager({
  instance,
  instanceId,
}: InstanceDataManagerProps) {
  const [loadOrder, setLoadOrder] = useState<LoadOrder | null>(null);
  const [initialLoading, setInitialLoading] = useState(true);
  const [scanning, setScanning] = useState(false);
  const [saving, setSaving] = useState(false);
  const [hosting, setHosting] = useState(false);
  const [pendingDeleteEntry, setPendingDeleteEntry] =
    useState<LoadOrderEntry | null>(null);
  const [deletingEntryId, setDeletingEntryId] = useState<string | null>(null);
  const [importingMo2, setImportingMo2] = useState(false);

  const clearMo2ContentOrder = (order: LoadOrder): LoadOrder => ({
    ...order,
    contentOrder: null,
  });

  const refresh = useCallback(async () => {
    if (!instanceId) return;
    setScanning(true);
    try {
      const order = await invoke<LoadOrder>("scan_instance_data", {
        instanceId,
      });
      setLoadOrder(order);
    } catch (error) {
      toast.error(`Failed to scan data directory: ${error}`);
    } finally {
      setInitialLoading(false);
      setScanning(false);
    }
  }, [instanceId]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const saveLoadOrder = async () => {
    if (!loadOrder) return;
    setSaving(true);
    try {
      const normalized = {
        ...loadOrder,
        entries: normalizePriorities(loadOrder.entries),
      };
      await invoke("save_instance_load_order", {
        instanceId,
        loadOrder: normalized,
      });
      setLoadOrder(normalized);
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

  const reorderEntries = useCallback((fromIndex: number, toIndex: number) => {
    setLoadOrder((current) => {
      if (!current) return current;
      return clearMo2ContentOrder({
        ...current,
        entries: reorderLoadOrderEntries(current.entries, fromIndex, toIndex),
      });
    });
  }, []);

  const moveEntry = useCallback((index: number, direction: -1 | 1) => {
    setLoadOrder((current) => {
      if (!current) return current;
      return clearMo2ContentOrder({
        ...current,
        entries: moveEntryByOffset(current.entries, index, direction),
      });
    });
  }, []);

  const updateEntryPriority = (entryId: string, rawPriority: string) => {
    setLoadOrder((current) => {
      if (!current) return current;
      return clearMo2ContentOrder({
        ...current,
        entries: setEntryPriority(current.entries, entryId, rawPriority),
      });
    });
  };

  const toggleEntry = (id: string) => {
    if (!loadOrder) return;
    setLoadOrder(
      clearMo2ContentOrder({
        ...loadOrder,
        entries: loadOrder.entries.map((e) =>
          e.id === id ? { ...e, enabled: !e.enabled } : e,
        ),
      }),
    );
  };

  const deleteEntry = async (entry: LoadOrderEntry) => {
    setDeletingEntryId(entry.id);
    try {
      const updated = await invoke<LoadOrder>("delete_instance_package", {
        instanceId,
        entryId: entry.id,
      });
      setLoadOrder(updated);
      setPendingDeleteEntry(null);
      toast.success(`Deleted "${entry.name}"`);
    } catch (error) {
      toast.error(`Failed to delete directory: ${error}`);
    } finally {
      setDeletingEntryId(null);
    }
  };

  const togglePlugin = (entryId: string, file: string) => {
    if (!loadOrder) return;
    setLoadOrder(
      clearMo2ContentOrder({
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
      }),
    );
  };

  const movePlugin = (entryId: string, index: number, direction: -1 | 1) => {
    if (!loadOrder) return;
    setLoadOrder(
      clearMo2ContentOrder({
        ...loadOrder,
        entries: loadOrder.entries.map((entry) => {
          if (entry.id !== entryId) return entry;
          const plugins = [...entry.plugins];
          const target = index + direction;
          if (target < 0 || target >= plugins.length) return entry;
          [plugins[index], plugins[target]] = [plugins[target], plugins[index]];
          return { ...entry, plugins };
        }),
      }),
    );
  };

  const importMo2Modlist = async () => {
    setImportingMo2(true);
    try {
      const csvPath = await invoke<string>("open_csv_file_picker");
      const result = await invoke<Mo2ModlistImportResult>("import_mo2_modlist_csv", {
        instanceId,
        csvPath,
      });
      setLoadOrder(result.loadOrder);

      const { report } = result;
      toast.success(
        `Imported MO2 load order (${report.matchedMods} mods, ${report.importedPlugins} plugins)`,
      );

      if (report.missingModDirectories.length > 0) {
        toast.warning(
          `${report.missingModDirectories.length} CSV mod folders were not found after rescan`,
        );
      }
      if (report.missingPlugins.length > 0) {
        toast.warning(
          `${report.missingPlugins.length} CSV plugins were not found on disk`,
        );
      }
    } catch (error) {
      const message = String(error);
      if (message.includes("No file selected")) {
        return;
      }
      toast.error(`Failed to import MO2 CSV: ${error}`);
    } finally {
      setImportingMo2(false);
    }
  };

  const sortedEntries = loadOrder
    ? sortLoadOrderEntries(loadOrder.entries)
    : [];

  return (
    <div className="mx-auto flex w-full max-w-3xl flex-col gap-5 px-1 pb-10">
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

      <Card className={INSTANCE_DATA_CARD_CLASS} disableHover disableTap>
        <CardHeader className="border-b border-border/50 space-y-2 pb-4 pt-5">
          <CardTitle className="font-display text-3xl tracking-[0.08em] text-gradient-gold">
            Data manager
          </CardTitle>
          <CardDescription className="font-serif text-left text-base leading-relaxed text-foreground/75">
            {instance.name} — order mod folders and plugins, then publish a
            manifest for sync.
          </CardDescription>
        </CardHeader>

        <CardContent className="flex flex-col gap-4 py-5">
          <DataManagerToolbar
            scanning={scanning}
            loadOrder={loadOrder}
            saving={saving}
            hosting={hosting}
            importingMo2={importingMo2}
            onRefresh={() => void refresh()}
            onImportMo2={() => void importMo2Modlist()}
            onSave={() => void saveLoadOrder()}
            onWriteLaunchCfg={() => void writeLaunchCfg()}
            onStartHosting={() => void startHosting()}
          />

          {loadOrder?.baseGameData ? (
            <p className="font-mono text-sm text-foreground/60 truncate">
              Base game: {loadOrder.baseGameData}
            </p>
          ) : null}

          <Tabs defaultValue="directories">
            <TabsList variant="line" className="w-full justify-start">
              <TabsTrigger value="directories">Directories</TabsTrigger>
              <TabsTrigger value="plugins">Plugins</TabsTrigger>
            </TabsList>

            <TabsContent value="directories" className="mt-4">
              <DirectoryList
                initialLoading={initialLoading}
                scanning={scanning}
                loadOrder={loadOrder}
                sortedEntries={sortedEntries}
                deletingEntryId={deletingEntryId}
                onToggle={toggleEntry}
                onMoveUp={(index) => moveEntry(index, -1)}
                onMoveDown={(index) => moveEntry(index, 1)}
                onReorder={reorderEntries}
                onPriorityChange={updateEntryPriority}
                onRequestDelete={setPendingDeleteEntry}
              />
            </TabsContent>

            <TabsContent value="plugins" className="mt-4">
              <div className="max-h-[min(60vh,720px)] overflow-y-auto rounded-lg border border-border/40 pr-1">
                <PluginList
                  sortedEntries={sortedEntries}
                  onTogglePlugin={togglePlugin}
                  onMovePlugin={movePlugin}
                />
              </div>
            </TabsContent>
          </Tabs>
        </CardContent>
      </Card>

      <DeletePackageDialog
        entry={pendingDeleteEntry}
        deleting={pendingDeleteEntry?.id === deletingEntryId}
        onOpenChange={(open) => {
          if (!open && pendingDeleteEntry?.id !== deletingEntryId) {
            setPendingDeleteEntry(null);
          }
        }}
        onConfirm={() => {
          if (pendingDeleteEntry) {
            void deleteEntry(pendingDeleteEntry);
          }
        }}
      />
    </div>
  );
}

function DataManagerToolbar({
  scanning,
  loadOrder,
  saving,
  hosting,
  importingMo2,
  onRefresh,
  onImportMo2,
  onSave,
  onWriteLaunchCfg,
  onStartHosting,
}: {
  scanning: boolean;
  loadOrder: LoadOrder | null;
  saving: boolean;
  hosting: boolean;
  importingMo2: boolean;
  onRefresh: () => void;
  onImportMo2: () => void;
  onSave: () => void;
  onWriteLaunchCfg: () => void;
  onStartHosting: () => void;
}) {
  return (
    <div className="flex flex-wrap gap-2">
      <Button
        variant="outline"
        className="h-10 text-base"
        disabled={scanning}
        onClick={onRefresh}
      >
        {scanning ? (
          <Loader2 className="animate-spin" data-icon="inline-start" />
        ) : (
          <RefreshCw data-icon="inline-start" />
        )}
        {scanning ? "Scanning…" : "Rescan"}
      </Button>
      <Button
        variant="outline"
        className="h-10 text-base"
        disabled={scanning || importingMo2}
        onClick={onImportMo2}
      >
        {importingMo2 ? (
          <Loader2 className="animate-spin" data-icon="inline-start" />
        ) : (
          <Upload data-icon="inline-start" />
        )}
        Import MO2 CSV
      </Button>
      <Button
        variant="outline"
        className="h-10 text-base"
        disabled={!loadOrder || saving}
        onClick={onSave}
      >
        <Save data-icon="inline-start" />
        Save order
      </Button>
      <Button
        variant="secondary"
        className="h-10 text-base"
        disabled={!loadOrder}
        onClick={onWriteLaunchCfg}
      >
        Write launch cfg
      </Button>
      <Button
        variant="launch"
        className="h-10 text-base"
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

function DeletePackageDialog({
  entry,
  deleting,
  onOpenChange,
  onConfirm,
}: {
  entry: LoadOrderEntry | null;
  deleting: boolean;
  onOpenChange: (open: boolean) => void;
  onConfirm: () => void;
}) {
  return (
    <AlertDialog open={entry !== null} onOpenChange={onOpenChange}>
      <AlertDialogContent className="data-[size=default]:max-w-md data-[size=default]:sm:max-w-lg">
        <AlertDialogHeader className="text-left">
          <AlertDialogTitle className="font-display text-xl tracking-[0.06em]">
            Delete mod directory?
          </AlertDialogTitle>
          <AlertDialogDescription asChild>
            <div className="space-y-3 text-left text-sm leading-relaxed text-muted-foreground">
              <p>
                This permanently removes the folder and every file inside it
                from this instance. This cannot be undone.
              </p>
              {entry ? (
                <>
                  <div className="rounded-md border border-destructive/30 bg-destructive/5 px-3 py-2">
                    <p className="font-display text-base text-foreground">
                      {entry.name}
                    </p>
                    <p className="font-mono text-xs text-foreground/70">
                      {entry.relativeDir}
                    </p>
                  </div>
                  <p>
                    If a manifest is published for this instance, it will be
                    updated to match. Synced clients will remove this package on
                    their next sync.
                  </p>
                </>
              ) : null}
            </div>
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel disabled={deleting}>Cancel</AlertDialogCancel>
          <AlertDialogAction
            variant="destructive"
            className="text-primary-foreground"
            disabled={deleting}
            onClick={(event) => {
              event.preventDefault();
              onConfirm();
            }}
          >
            {deleting ? (
              <>
                <Loader2 className="animate-spin" data-icon="inline-start" />
                Deleting…
              </>
            ) : (
              "Delete permanently"
            )}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}

function DirectoryList({
  initialLoading,
  scanning,
  loadOrder,
  sortedEntries,
  deletingEntryId,
  onToggle,
  onMoveUp,
  onMoveDown,
  onReorder,
  onPriorityChange,
  onRequestDelete,
}: {
  initialLoading: boolean;
  scanning: boolean;
  loadOrder: LoadOrder | null;
  sortedEntries: LoadOrderEntry[];
  deletingEntryId: string | null;
  onToggle: (id: string) => void;
  onMoveUp: (index: number) => void;
  onMoveDown: (index: number) => void;
  onReorder: (fromIndex: number, toIndex: number) => void;
  onPriorityChange: (entryId: string, rawPriority: string) => void;
  onRequestDelete: (entry: LoadOrderEntry) => void;
}) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const [activeEntryId, setActiveEntryId] = useState<string | null>(null);
  const [overlayWidth, setOverlayWidth] = useState<number | undefined>();

  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: { distance: 6 },
    }),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    }),
  );

  const activeEntry = activeEntryId
    ? sortedEntries.find((entry) => entry.id === activeEntryId)
    : null;

  const handleDragStart = (event: DragStartEvent) => {
    setOverlayWidth(scrollRef.current?.clientWidth);
    setActiveEntryId(String(event.active.id));
  };

  const handleDragEnd = (event: DragEndEvent) => {
    setActiveEntryId(null);
    setOverlayWidth(undefined);
    const { active, over } = event;
    if (!over || active.id === over.id) return;

    const fromIndex = sortedEntries.findIndex(
      (entry) => entry.id === active.id,
    );
    const toIndex = sortedEntries.findIndex((entry) => entry.id === over.id);
    if (fromIndex < 0 || toIndex < 0) return;

    onReorder(fromIndex, toIndex);
  };

  const handleDragCancel = () => {
    setActiveEntryId(null);
    setOverlayWidth(undefined);
  };

  if (initialLoading && !loadOrder) {
    return <p className="font-serif text-base text-foreground/65">Scanning…</p>;
  }

  if (sortedEntries.length === 0 && !scanning) {
    return (
      <p className="font-serif text-base leading-relaxed text-foreground/65">
        No packages found. Add mod folders directly in the instance data
        directory, then rescan.
      </p>
    );
  }

  if (sortedEntries.length === 0 && scanning) {
    return (
      <p className="font-serif text-base text-foreground/65">
        Rescanning data directory…
      </p>
    );
  }

  return (
    <div className="space-y-2">
      {scanning ? (
        <p className="flex items-center gap-2 font-serif text-sm text-foreground/70">
          <Loader2 className="size-4 animate-spin" />
          Rescanning data directory…
        </p>
      ) : null}
    <DndContext
      sensors={sensors}
      collisionDetection={closestCenter}
      onDragStart={handleDragStart}
      onDragEnd={handleDragEnd}
      onDragCancel={handleDragCancel}
    >
      <div
        ref={scrollRef}
        className="max-h-[min(60vh,720px)] overflow-y-auto rounded-lg border border-border/40 pr-1"
      >
        <SortableContext
          items={sortedEntries.map((entry) => entry.id)}
          strategy={verticalListSortingStrategy}
        >
          <div className="space-y-2 p-1">
            {sortedEntries.map((entry, index) => (
              <SortableDirectoryRow
                key={entry.id}
                entry={entry}
                index={index}
                total={sortedEntries.length}
                deleting={deletingEntryId === entry.id}
                onToggle={() => onToggle(entry.id)}
                onMoveUp={() => onMoveUp(index)}
                onMoveDown={() => onMoveDown(index)}
                onPriorityChange={(value) => onPriorityChange(entry.id, value)}
                onRequestDelete={() => onRequestDelete(entry)}
              />
            ))}
          </div>
        </SortableContext>
      </div>

      <DragOverlay dropAnimation={{ duration: 180, easing: "ease-out" }}>
        {activeEntry ? (
          <div style={{ width: overlayWidth }} className="max-w-full">
            <DirectoryRowShell
              entry={activeEntry}
              index={sortedEntries.findIndex(
                (entry) => entry.id === activeEntry.id,
              )}
              total={sortedEntries.length}
              isOverlay
              onToggle={() => {}}
              onMoveUp={() => {}}
              onMoveDown={() => {}}
              onPriorityChange={() => {}}
              onRequestDelete={() => {}}
            />
          </div>
        ) : null}
      </DragOverlay>
    </DndContext>
    </div>
  );
}

function SortableDirectoryRow({
  entry,
  index,
  total,
  deleting,
  onToggle,
  onMoveUp,
  onMoveDown,
  onPriorityChange,
  onRequestDelete,
}: {
  entry: LoadOrderEntry;
  index: number;
  total: number;
  deleting: boolean;
  onToggle: () => void;
  onMoveUp: () => void;
  onMoveDown: () => void;
  onPriorityChange: (rawPriority: string) => void;
  onRequestDelete: () => void;
}) {
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id: entry.id });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
  };

  return (
    <div
      ref={setNodeRef}
      style={style}
      className={cn(isDragging && "opacity-40")}
    >
      <DirectoryRowShell
        entry={entry}
        index={index}
        total={total}
        deleting={deleting}
        dragHandleProps={{ ...attributes, ...listeners }}
        onToggle={onToggle}
        onMoveUp={onMoveUp}
        onMoveDown={onMoveDown}
        onPriorityChange={onPriorityChange}
        onRequestDelete={onRequestDelete}
      />
    </div>
  );
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
    <div className="space-y-4 p-1">
      {withPlugins.map((entry) => (
        <div
          key={entry.id}
          className="rounded-lg border border-border/50 bg-background/20 p-3"
        >
          <div className="mb-2 flex items-center gap-2">
            <span className="font-display text-sm tracking-[0.15em] text-accent uppercase">
              {entry.name}
            </span>
            {!entry.enabled ? (
              <Badge variant="secondary" className="text-xs">
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
                <label className="flex flex-1 cursor-pointer items-center gap-2 font-mono text-sm">
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
                    variant="outline"
                    size="icon-sm"
                    disabled={pluginIndex === 0}
                    onClick={() => onMovePlugin(entry.id, pluginIndex, -1)}
                  >
                    <ArrowUp />
                  </Button>
                  <Button
                    variant="outline"
                    size="icon-sm"
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
        <p className="font-serif text-base text-foreground/65">
          No plugins detected in scanned folders.
        </p>
      ) : null}
    </div>
  );
}

function PriorityInput({
  value,
  disabled,
  onCommit,
}: {
  value: number;
  disabled?: boolean;
  onCommit: (rawPriority: string) => void;
}) {
  const [draft, setDraft] = useState(String(value));

  useEffect(() => {
    setDraft(String(value));
  }, [value]);

  const commitIfChanged = () => {
    const parsed = Number.parseInt(draft, 10);
    if (Number.isNaN(parsed) || parsed < 1 || parsed === value) {
      setDraft(String(value));
      return;
    }
    onCommit(draft);
  };

  return (
    <Input
      type="number"
      min={1}
      inputMode="numeric"
      aria-label="Load order priority"
      disabled={disabled}
      value={draft}
      onChange={(event) => setDraft(event.target.value)}
      onBlur={commitIfChanged}
      onKeyDown={(event) => {
        if (event.key === "Enter") {
          event.currentTarget.blur();
        }
      }}
      onMouseDown={(event) => event.stopPropagation()}
      className="h-9 w-[4.5rem] shrink-0 px-2 text-center font-mono text-sm"
    />
  );
}

function TruncatedText({
  text,
  className,
}: {
  text: string;
  className?: string;
}) {
  const ref = useRef<HTMLSpanElement>(null);
  const [truncated, setTruncated] = useState(false);

  useEffect(() => {
    const element = ref.current;
    if (!element) return;

    const measure = () => {
      setTruncated(element.scrollWidth > element.clientWidth + 1);
    };

    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    return () => observer.disconnect();
  }, [text]);

  return (
    <span
      ref={ref}
      className={cn("block truncate", className)}
      title={truncated ? text : undefined}
    >
      {text}
    </span>
  );
}

function DirectoryRowLabels({
  title,
  subtitle,
}: {
  title: string;
  subtitle: string;
}) {
  return (
    <div className="min-w-0 flex-1">
      <TruncatedText
        text={title}
        className="font-display text-base text-foreground"
      />
      <TruncatedText
        text={subtitle}
        className="font-mono text-sm text-foreground/55"
      />
    </div>
  );
}

function DirectoryRowShell({
  entry,
  index,
  total,
  dragHandleProps,
  isOverlay = false,
  deleting = false,
  onToggle,
  onMoveUp,
  onMoveDown,
  onPriorityChange,
  onRequestDelete,
}: {
  entry: LoadOrderEntry;
  index: number;
  total: number;
  dragHandleProps?: HTMLAttributes<HTMLButtonElement>;
  isOverlay?: boolean;
  deleting?: boolean;
  onToggle: () => void;
  onMoveUp: () => void;
  onMoveDown: () => void;
  onPriorityChange: (rawPriority: string) => void;
  onRequestDelete: () => void;
}) {
  return (
    <div
      data-directory-row
      className={cn(
        "flex items-center gap-2 rounded-lg border border-border/50 bg-card px-3 py-3 transition-colors",
        !entry.enabled && "opacity-50",
        deleting && "opacity-60",
        isOverlay &&
          "cursor-grabbing shadow-[0_12px_40px_hsl(var(--background)/0.45)] ring-2 ring-accent/40",
      )}
    >
      <button
        type="button"
        aria-label="Drag to reorder"
        className="inline-flex size-9 shrink-0 cursor-grab touch-none items-center justify-center rounded-md text-foreground/45 hover:bg-muted/40 hover:text-foreground/75 active:cursor-grabbing"
        {...dragHandleProps}
      >
        <GripVertical className="size-4" />
      </button>
      <label className="flex min-w-0 flex-1 cursor-pointer items-center gap-2">
        <input
          type="checkbox"
          checked={entry.enabled}
          onChange={onToggle}
          disabled={isOverlay}
        />
        <DirectoryRowLabels title={entry.name} subtitle={entry.relativeDir} />
      </label>
      <PriorityInput
        value={entry.priority}
        disabled={isOverlay}
        onCommit={onPriorityChange}
      />
      <Badge
        variant="outline"
        className="hidden shrink-0 text-xs uppercase sm:inline-flex"
      >
        {entry.kind}
      </Badge>
      <div className="flex shrink-0 gap-1">
        <Button
          type="button"
          variant="outline"
          size="icon-sm"
          disabled={isOverlay || deleting || index === 0}
          onClick={onMoveUp}
        >
          <ArrowUp />
        </Button>
        <Button
          type="button"
          variant="outline"
          size="icon-sm"
          disabled={isOverlay || deleting || index === total - 1}
          onClick={onMoveDown}
        >
          <ArrowDown />
        </Button>
        <Button
          type="button"
          variant="outline"
          size="icon-sm"
          disabled={isOverlay || deleting}
          aria-label={`Delete ${entry.name}`}
          className="text-destructive hover:bg-destructive/10 hover:text-destructive"
          onClick={onRequestDelete}
        >
          {deleting ? <Loader2 className="animate-spin" /> : <Trash2 />}
        </Button>
      </div>
    </div>
  );
}
