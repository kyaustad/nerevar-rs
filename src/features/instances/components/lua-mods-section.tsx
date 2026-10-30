import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectItemText,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type {
  InstanceSettings,
  LuaModInfo,
  LuaScriptInfo,
  ScriptMode,
  ScriptModeSuggestion,
} from "@/types";
import { invoke } from "@tauri-apps/api/core";
import { AlertTriangle, Loader2, RefreshCw, Sparkles, X } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { toast } from "sonner";

const MODE_LABEL: Record<ScriptMode, string> = {
  client: "Client",
  authority: "Authority",
};

const CONFIDENCE_LABEL = {
  high: "likely",
  medium: "probably",
  low: "unsure",
} as const;

/** Policy key for a script: OpenMW's VFS paths are case-insensitive. */
function policyKey(path: string): string {
  return path.toLowerCase();
}

function findPolicyMode(
  settings: InstanceSettings,
  path: string,
): ScriptMode | undefined {
  const key = policyKey(path);
  const entry = Object.entries(settings.luaScriptPolicy.scripts).find(
    ([script]) => policyKey(script) === key,
  );
  return entry?.[1];
}

function setPolicyMode(
  settings: InstanceSettings,
  path: string,
  mode: ScriptMode | undefined,
): InstanceSettings {
  const key = policyKey(path);
  const scripts = Object.fromEntries(
    Object.entries(settings.luaScriptPolicy.scripts).filter(
      ([script]) => policyKey(script) !== key,
    ),
  );
  if (mode) {
    scripts[path] = mode;
  }
  return {
    ...settings,
    luaScriptPolicy: { ...settings.luaScriptPolicy, scripts },
  };
}

function SuggestionDetails({
  suggestion,
}: {
  suggestion: ScriptModeSuggestion;
}) {
  return (
    <div className="space-y-1 font-serif text-sm text-foreground/65">
      <ul className="list-disc space-y-0.5 pl-5">
        {suggestion.reasons.map((reason) => (
          <li key={reason}>{reason}</li>
        ))}
      </ul>
      {suggestion.warnings.map((warning) => (
        <p key={warning} className="flex items-start gap-1.5 text-amber-400/90">
          <AlertTriangle className="mt-0.5 size-3.5 shrink-0" />
          {warning}
        </p>
      ))}
    </div>
  );
}

function GlobalScriptRow({
  script,
  settings,
  readOnly,
  onChange,
}: {
  script: LuaScriptInfo;
  settings: InstanceSettings;
  readOnly?: boolean;
  onChange: (settings: InstanceSettings) => void;
}) {
  const explicit = findPolicyMode(settings, script.path);
  const defaultMode = settings.luaScriptPolicy.defaultMode;
  const suggestion = script.suggestion ?? null;
  const effective = explicit ?? defaultMode;
  const matchesSuggestion = suggestion ? suggestion.mode === effective : true;

  return (
    <div className="space-y-2 rounded-lg border border-border/50 bg-background/20 px-4 py-3">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="min-w-0 space-y-1">
          <p className="break-all font-mono text-sm text-foreground">
            {script.path}
          </p>
          <div className="flex flex-wrap items-center gap-1.5">
            {!script.found ? (
              <Badge
                variant="outline"
                className="border-destructive/50 text-destructive"
              >
                File not found
              </Badge>
            ) : null}
            {suggestion ? (
              <Badge
                variant="outline"
                className={
                  matchesSuggestion
                    ? "border-emerald-500/40 text-emerald-400"
                    : "border-amber-500/50 text-amber-400"
                }
              >
                Suggested: {MODE_LABEL[suggestion.mode]} (
                {CONFIDENCE_LABEL[suggestion.confidence]})
              </Badge>
            ) : null}
          </div>
        </div>
        <Select
          value={explicit ?? "default"}
          disabled={readOnly}
          onValueChange={(value) =>
            onChange(
              setPolicyMode(
                settings,
                script.path,
                value === "default" ? undefined : (value as ScriptMode),
              ),
            )
          }
        >
          <SelectTrigger className="w-48">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="default">
              <SelectItemText>
                Default ({MODE_LABEL[defaultMode].toLowerCase()})
              </SelectItemText>
            </SelectItem>
            <SelectItem value="client">
              <SelectItemText>Client</SelectItemText>
            </SelectItem>
            <SelectItem value="authority">
              <SelectItemText>Authority</SelectItemText>
            </SelectItem>
          </SelectContent>
        </Select>
      </div>
      {suggestion ? <SuggestionDetails suggestion={suggestion} /> : null}
    </div>
  );
}

function LuaModCard({
  mod,
  settings,
  readOnly,
  onChange,
}: {
  mod: LuaModInfo;
  settings: InstanceSettings;
  readOnly?: boolean;
  onChange: (settings: InstanceSettings) => void;
}) {
  const globals = mod.scripts.filter((script) => script.isGlobal);
  const locals = mod.scripts.filter((script) => !script.isGlobal);

  return (
    <div className="space-y-3 rounded-lg border border-border/60 bg-card/40 p-4">
      <div className="flex flex-wrap items-start justify-between gap-2">
        <div className="min-w-0">
          <p className="font-display text-sm tracking-[0.08em] text-foreground">
            {mod.file}
          </p>
          <p className="break-all font-mono text-xs text-foreground/50">
            {mod.dataPath}
          </p>
        </div>
        {mod.enabled ? (
          <Badge variant="outline" className="border-accent/40 text-accent">
            In load order
          </Badge>
        ) : (
          <Badge variant="outline" className="text-foreground/50">
            Not in load order
          </Badge>
        )}
      </div>

      {globals.length > 0 ? (
        <div className="flex flex-col gap-2">
          {globals.map((script) => (
            <GlobalScriptRow
              key={script.path}
              script={script}
              settings={settings}
              readOnly={readOnly}
              onChange={onChange}
            />
          ))}
        </div>
      ) : (
        <p className="font-serif text-sm text-foreground/60">
          No global scripts: nothing to configure. Its scripts run on each
          player&apos;s game.
        </p>
      )}

      {locals.length > 0 ? (
        <div className="space-y-1">
          <p className="font-display text-xs tracking-[0.12em] text-foreground/50 uppercase">
            Other scripts (run where their object is simulated)
          </p>
          <ul className="space-y-0.5 font-mono text-xs text-foreground/60">
            {locals.map((script) => (
              <li key={`${script.tags.join(",")}:${script.path}`}>
                <span className="text-accent/70">{script.tags.join(", ")}</span>{" "}
                {script.path}
                {!script.found ? (
                  <span className="text-destructive"> (not found)</span>
                ) : null}
              </li>
            ))}
          </ul>
        </div>
      ) : null}
    </div>
  );
}

export function LuaModsSection({
  instanceId,
  settings,
  readOnly,
  onChange,
}: {
  instanceId: string;
  settings: InstanceSettings;
  readOnly?: boolean;
  onChange: (settings: InstanceSettings) => void;
}) {
  const [mods, setMods] = useState<LuaModInfo[] | null>(null);
  const [scanning, setScanning] = useState(false);

  const scan = useCallback(async () => {
    setScanning(true);
    try {
      setMods(
        await invoke<LuaModInfo[]>("scan_instance_lua_mods", { instanceId }),
      );
    } catch (error) {
      toast.error(`Failed to read Lua mods: ${error}`);
    } finally {
      setScanning(false);
    }
  }, [instanceId]);

  useEffect(() => {
    void scan();
  }, [scan]);

  const globalScripts = useMemo(
    () =>
      (mods ?? []).flatMap((mod) =>
        mod.scripts.filter((script) => script.isGlobal),
      ),
    [mods],
  );

  // Policy entries for scripts no mod has anymore
  const unusedEntries = useMemo(() => {
    if (!mods) return [];
    const known = new Set(
      globalScripts.map((script) => policyKey(script.path)),
    );
    return Object.keys(settings.luaScriptPolicy.scripts).filter(
      (script) => !known.has(policyKey(script)),
    );
  }, [mods, globalScripts, settings.luaScriptPolicy.scripts]);

  const applySuggestions = () => {
    let next = settings;
    let changed = 0;
    for (const script of globalScripts) {
      if (!script.suggestion) continue;
      const mode = script.suggestion.mode;
      const effective =
        findPolicyMode(next, script.path) ?? next.luaScriptPolicy.defaultMode;
      if (effective !== mode) changed += 1;
      next = setPolicyMode(
        next,
        script.path,
        mode === next.luaScriptPolicy.defaultMode ? undefined : mode,
      );
    }
    onChange(next);
    toast.success(
      changed > 0
        ? `Applied ${changed} suggestion(s). Save settings to keep them.`
        : "Every global script already follows its suggestion",
    );
  };

  return (
    <div className="space-y-5">
      <div className="space-y-1">
        <h3 className="font-display text-sm tracking-[0.12em] text-accent uppercase">
          Lua mods
        </h3>
        <p className="font-serif text-sm leading-relaxed text-foreground/65">
          OpenMW Lua mods (<code>.omwscripts</code>) are content files: enable
          them in the load order like plugins. Their <strong>global</strong>{" "}
          scripts can run on every player&apos;s game (<strong>client</strong>,
          as in single player) or once, on one player&apos;s game, for everyone
          (<strong>authority</strong>). Scripts that change the shared world on
          their own (spawners, timed events, economies) need authority; scripts
          that react to a player&apos;s action belong on that player&apos;s
          client. Saved to your server&apos;s{" "}
          <code>data/lua/scriptPolicy.json</code>.
        </p>
      </div>

      <div className="flex flex-wrap items-center justify-between gap-3 rounded-lg border border-border/50 bg-background/20 px-4 py-3">
        <div className="space-y-1">
          <p className="font-display text-sm tracking-[0.08em] text-foreground">
            Default mode
          </p>
          <p className="font-serif text-sm text-foreground/65">
            For global scripts without their own mode.
          </p>
        </div>
        <Select
          value={settings.luaScriptPolicy.defaultMode}
          disabled={readOnly}
          onValueChange={(value) =>
            onChange({
              ...settings,
              luaScriptPolicy: {
                ...settings.luaScriptPolicy,
                defaultMode: value as ScriptMode,
              },
            })
          }
        >
          <SelectTrigger className="w-48">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="client">
              <SelectItemText>Client</SelectItemText>
            </SelectItem>
            <SelectItem value="authority">
              <SelectItemText>Authority</SelectItemText>
            </SelectItem>
          </SelectContent>
        </Select>
      </div>

      <div className="flex flex-wrap gap-2">
        <Button
          type="button"
          variant="outline"
          disabled={scanning}
          onClick={() => void scan()}
        >
          {scanning ? (
            <Loader2 className="animate-spin" data-icon="inline-start" />
          ) : (
            <RefreshCw data-icon="inline-start" />
          )}
          Rescan mods
        </Button>
        {!readOnly ? (
          <Button
            type="button"
            variant="outline"
            disabled={scanning || globalScripts.length === 0}
            onClick={applySuggestions}
          >
            <Sparkles data-icon="inline-start" />
            Apply suggestions
          </Button>
        ) : null}
      </div>

      {mods === null ? (
        <div className="flex justify-center py-6">
          <Loader2 className="size-5 animate-spin text-accent" />
        </div>
      ) : mods.length === 0 ? (
        <p className="font-serif text-sm text-foreground/60">
          No Lua mods found in this instance&apos;s data folders.
        </p>
      ) : (
        <div className="flex flex-col gap-3">
          {mods.map((mod) => (
            <LuaModCard
              key={`${mod.dataPath}/${mod.file}`}
              mod={mod}
              settings={settings}
              readOnly={readOnly}
              onChange={onChange}
            />
          ))}
        </div>
      )}

      {unusedEntries.length > 0 ? (
        <div className="space-y-2 rounded-lg border border-amber-500/30 bg-amber-500/5 px-4 py-3">
          <p className="font-display text-sm tracking-[0.08em] text-amber-400">
            Modes for scripts no mod has
          </p>
          <ul className="space-y-1">
            {unusedEntries.map((script) => (
              <li
                key={script}
                className="flex items-center justify-between gap-2 font-mono text-xs text-foreground/70"
              >
                <span className="break-all">
                  {script}: {settings.luaScriptPolicy.scripts[script]}
                </span>
                {!readOnly ? (
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    onClick={() =>
                      onChange(setPolicyMode(settings, script, undefined))
                    }
                  >
                    <X data-icon="inline-start" />
                    Remove
                  </Button>
                ) : null}
              </li>
            ))}
          </ul>
        </div>
      ) : null}
    </div>
  );
}
