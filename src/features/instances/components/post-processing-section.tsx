import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectItemText,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Textarea } from "@/components/ui/textarea";
import {
  getOpenMwValue,
  setOpenMwValue,
  SettingField,
} from "@/features/instances/components/setting-field";
import { displaySettingValue, makeSettingValue } from "@/lib/setting-value";
import type {
  InstanceSettings,
  InstanceShaders,
  SettingDefinition,
} from "@/types";
import { invoke } from "@tauri-apps/api/core";
import {
  ArrowDown,
  ArrowUp,
  Download,
  Loader2,
  RefreshCw,
  X,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { toast } from "sonner";

const CHAIN_KEY = "chain";
const ENABLED_KEY = "enabled";

function parseChain(chain: string): string[] {
  return chain
    .split(",")
    .map((name) => name.trim())
    .filter((name) => name.length > 0);
}

export function PostProcessingSection({
  instanceId,
  definitions,
  settings,
  readOnly,
  onChange,
}: {
  instanceId: string;
  /** Settings of the openMwPostProcessing category */
  definitions: SettingDefinition[];
  settings: InstanceSettings;
  readOnly?: boolean;
  onChange: (settings: InstanceSettings) => void;
}) {
  const [scan, setScan] = useState<InstanceShaders | null>(null);
  const [scanning, setScanning] = useState(false);
  const [adding, setAdding] = useState("");

  const load = useCallback(async () => {
    setScanning(true);
    try {
      setScan(
        await invoke<InstanceShaders>("scan_instance_shaders", { instanceId }),
      );
    } catch (error) {
      toast.error(`Failed to find shaders: ${error}`);
    } finally {
      setScanning(false);
    }
  }, [instanceId]);

  useEffect(() => {
    void load();
  }, [load]);

  const chainDefinition = definitions.find((d) => d.key === CHAIN_KEY);
  const enabledDefinition = definitions.find((d) => d.key === ENABLED_KEY);
  const otherDefinitions = definitions.filter((d) => d.key !== CHAIN_KEY);

  const chain = useMemo(
    () =>
      chainDefinition
        ? parseChain(
            String(
              displaySettingValue(getOpenMwValue(settings, chainDefinition)),
            ),
          )
        : [],
    [settings, chainDefinition],
  );
  const available = scan?.shaders ?? [];
  const known = new Set(available.map((shader) => shader.name.toLowerCase()));
  const addable = available.filter(
    (shader) =>
      !chain.some((name) => name.toLowerCase() === shader.name.toLowerCase()),
  );

  const setChain = (next: string[], base: InstanceSettings = settings) => {
    if (!chainDefinition) return base;
    return setOpenMwValue(
      base,
      chainDefinition,
      makeSettingValue("string", next.join(",")),
    );
  };

  const move = (index: number, offset: number) => {
    const next = [...chain];
    const [item] = next.splice(index, 1);
    next.splice(index + offset, 0, item);
    onChange(setChain(next));
  };

  const hasGameSetup =
    scan !== null &&
    (scan.gameChain != null ||
      scan.gamePostProcessingEnabled != null ||
      scan.gameShadersYaml != null);

  const importFromGame = () => {
    if (!scan) return;
    let next = settings;
    if (scan.gameChain != null) {
      next = setChain(parseChain(scan.gameChain), next);
    }
    if (scan.gamePostProcessingEnabled != null && enabledDefinition) {
      next = setOpenMwValue(
        next,
        enabledDefinition,
        makeSettingValue("boolean", scan.gamePostProcessingEnabled),
      );
    }
    if (scan.gameShadersYaml != null) {
      next = { ...next, shadersYaml: scan.gameShadersYaml };
    }
    onChange(next);
    toast.success(
      "Imported your game's post-processing. Save settings to keep it.",
    );
  };

  return (
    <div className="space-y-5">
      <div className="space-y-1">
        <h3 className="font-display text-sm tracking-[0.12em] text-accent uppercase">
          Post-processing
        </h3>
        <p className="font-serif text-sm leading-relaxed text-foreground/65">
          OpenMW post-processing shaders (<code>.omwfx</code>) from TES3MP-NG
          and your mods. The chain and these settings are seeded into each
          player&apos;s <code>settings.cfg</code>, and shader values into their{" "}
          <code>shaders.yaml</code>; players can still change them, for example
          to turn shaders off for performance. Tip: tune shaders in your game
          with F2, then use <strong>Import from my game</strong>.
        </p>
      </div>

      <div className="flex flex-wrap gap-2">
        <Button
          type="button"
          variant="outline"
          disabled={scanning}
          onClick={() => void load()}
        >
          {scanning ? (
            <Loader2 className="animate-spin" data-icon="inline-start" />
          ) : (
            <RefreshCw data-icon="inline-start" />
          )}
          Rescan shaders
        </Button>
        {!readOnly ? (
          <Button
            type="button"
            variant="outline"
            disabled={!hasGameSetup}
            onClick={importFromGame}
          >
            <Download data-icon="inline-start" />
            Import from my game
          </Button>
        ) : null}
      </div>

      <div className="flex flex-col gap-3">
        {otherDefinitions.map((definition) => (
          <SettingField
            key={definition.key}
            definition={definition}
            value={getOpenMwValue(settings, definition)}
            disabled={readOnly}
            onChange={(value) =>
              onChange(setOpenMwValue(settings, definition, value))
            }
          />
        ))}
      </div>

      {chainDefinition ? (
        <div className="space-y-3 rounded-lg border border-border/50 bg-background/20 px-4 py-3">
          <div className="space-y-1">
            <Label className="font-display text-sm tracking-[0.08em] text-foreground">
              Shader chain
            </Label>
            <p className="font-serif text-sm text-foreground/65">
              Applied top to bottom, when post-processing is on.
            </p>
          </div>

          {chain.length === 0 ? (
            <p className="font-serif text-sm text-foreground/50">
              No shaders in the chain.
            </p>
          ) : (
            <ol className="flex flex-col gap-1.5">
              {chain.map((name, index) => {
                const shader = available.find(
                  (s) => s.name.toLowerCase() === name.toLowerCase(),
                );
                return (
                  <li
                    key={`${name}-${index}`}
                    className="flex items-center justify-between gap-2 rounded-md border border-border/40 bg-card/40 px-3 py-1.5"
                  >
                    <div className="flex min-w-0 items-center gap-2">
                      <span className="font-mono text-xs text-foreground/40">
                        {index + 1}.
                      </span>
                      <span className="font-mono text-sm text-foreground">
                        {name}
                      </span>
                      {shader ? (
                        <span className="truncate font-serif text-xs text-foreground/50">
                          {shader.source}
                        </span>
                      ) : scan && !known.has(name.toLowerCase()) ? (
                        <Badge
                          variant="outline"
                          className="border-amber-500/50 text-amber-400"
                        >
                          Not found in this instance
                        </Badge>
                      ) : null}
                    </div>
                    {!readOnly ? (
                      <div className="flex shrink-0 gap-1">
                        <Button
                          type="button"
                          variant="ghost"
                          size="icon-sm"
                          aria-label="Move up"
                          disabled={index === 0}
                          onClick={() => move(index, -1)}
                        >
                          <ArrowUp />
                        </Button>
                        <Button
                          type="button"
                          variant="ghost"
                          size="icon-sm"
                          aria-label="Move down"
                          disabled={index === chain.length - 1}
                          onClick={() => move(index, 1)}
                        >
                          <ArrowDown />
                        </Button>
                        <Button
                          type="button"
                          variant="ghost"
                          size="icon-sm"
                          aria-label="Remove"
                          onClick={() =>
                            onChange(
                              setChain(chain.filter((_, i) => i !== index)),
                            )
                          }
                        >
                          <X />
                        </Button>
                      </div>
                    ) : null}
                  </li>
                );
              })}
            </ol>
          )}

          {!readOnly ? (
            <Select
              value={adding}
              disabled={addable.length === 0}
              onValueChange={(name) => {
                setAdding("");
                onChange(setChain([...chain, name]));
              }}
            >
              <SelectTrigger className="w-full">
                <SelectValue
                  placeholder={
                    addable.length === 0
                      ? "No other shaders found"
                      : "Add a shader to the chain"
                  }
                />
              </SelectTrigger>
              <SelectContent position="popper" className="max-h-72">
                {addable.map((shader) => (
                  <SelectItem key={shader.path} value={shader.name}>
                    <SelectItemText>{shader.name}</SelectItemText>
                    <span className="text-foreground/50">
                      ({shader.source})
                    </span>
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          ) : null}
        </div>
      ) : null}

      <div className="space-y-3 rounded-lg border border-border/50 bg-background/20 px-4 py-3">
        <div className="space-y-1">
          <Label className="font-display text-sm tracking-[0.08em] text-foreground">
            Shader values (shaders.yaml)
          </Label>
          <p className="font-serif text-sm text-foreground/65">
            {settings.shadersYaml == null
              ? "Players get your game's shaders.yaml, as you tuned it in-game."
              : "Players get these values."}
          </p>
        </div>
        {settings.shadersYaml == null ? (
          !readOnly ? (
            <Button
              type="button"
              variant="outline"
              onClick={() =>
                onChange({
                  ...settings,
                  shadersYaml: scan?.gameShadersYaml ?? "",
                })
              }
            >
              Use fixed values instead
            </Button>
          ) : null
        ) : (
          <>
            <Textarea
              value={settings.shadersYaml}
              disabled={readOnly}
              className="min-h-48 font-mono text-sm"
              placeholder={"hdr:\n  exposure: 1.0"}
              onChange={(event) =>
                onChange({ ...settings, shadersYaml: event.target.value })
              }
            />
            {!readOnly ? (
              <Button
                type="button"
                variant="outline"
                onClick={() => onChange({ ...settings, shadersYaml: null })}
              >
                Share my game&apos;s shaders.yaml instead
              </Button>
            ) : null}
          </>
        )}
      </div>
    </div>
  );
}
