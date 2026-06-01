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
import { Switch } from "@/components/ui/switch";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Textarea } from "@/components/ui/textarea";
import { INSTANCE_DATA_CARD_CLASS } from "@/features/instances/components/instance-data-manager";
import { displaySettingValue, makeSettingValue } from "@/lib/setting-value";
import type {
  InstanceSettings,
  SettingDefinition,
  SettingValue,
} from "@/types";
import { invoke } from "@tauri-apps/api/core";
import { Loader2, Save, ServerCog } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { toast } from "sonner";

type InstanceServerSettingsManagerProps = {
  instanceId: string;
  instanceName: string;
  readOnly?: boolean;
};

function getTes3mpValue(
  settings: InstanceSettings,
  name: string,
  fallback: SettingValue,
): SettingValue {
  return (
    settings.tes3mpGameSettings.find((entry) => entry.name === name)?.value ??
    fallback
  );
}

function setTes3mpValue(
  settings: InstanceSettings,
  name: string,
  value: SettingValue,
): InstanceSettings {
  const nextEntries = settings.tes3mpGameSettings.filter(
    (entry) => entry.name !== name,
  );
  nextEntries.push({ name, value });
  nextEntries.sort((a, b) => a.name.localeCompare(b.name));
  return { ...settings, tes3mpGameSettings: nextEntries };
}

function openMwSection(definition: SettingDefinition): string {
  if (definition.category === "openMwGraphics") return "Graphics";
  if (definition.category === "openMwShaders") return "Shaders";
  return "Game";
}

function getOpenMwValue(
  settings: InstanceSettings,
  definition: SettingDefinition,
): SettingValue {
  const section = openMwSection(definition);
  return (
    settings.openmwSettings[section]?.[definition.key] ??
    definition.defaultValue
  );
}

function setOpenMwValue(
  settings: InstanceSettings,
  definition: SettingDefinition,
  value: SettingValue,
): InstanceSettings {
  const section = openMwSection(definition);
  return {
    ...settings,
    openmwSettings: {
      ...settings.openmwSettings,
      [section]: {
        ...(settings.openmwSettings[section] ?? {}),
        [definition.key]: value,
      },
    },
  };
}

function SettingField({
  definition,
  value,
  disabled,
  onChange,
}: {
  definition: SettingDefinition;
  value: SettingValue;
  disabled?: boolean;
  onChange: (value: SettingValue) => void;
}) {
  if (definition.valueType === "boolean") {
    return (
      <div className="flex items-start justify-between gap-4 rounded-lg border border-border/50 bg-background/20 px-4 py-3">
        <div className="space-y-1">
          <Label className="font-display text-sm tracking-[0.08em] text-foreground">
            {definition.label}
          </Label>
          <p className="font-serif text-sm leading-relaxed text-foreground/65">
            {definition.description}
          </p>
        </div>
        <Switch
          checked={displaySettingValue(value) === true}
          disabled={disabled}
          onCheckedChange={(checked) =>
            onChange(makeSettingValue("boolean", checked))
          }
        />
      </div>
    );
  }

  return (
    <div className="space-y-2 rounded-lg border border-border/50 bg-background/20 px-4 py-3">
      <Label className="font-display text-sm tracking-[0.08em] text-foreground">
        {definition.label}
      </Label>
      <p className="font-serif text-sm leading-relaxed text-foreground/65">
        {definition.description}
      </p>
      <Input
        type={definition.valueType === "integer" ? "number" : "text"}
        value={String(displaySettingValue(value))}
        disabled={disabled}
        min={definition.minInteger ?? undefined}
        max={definition.maxInteger ?? undefined}
        className="font-mono"
        onChange={(event) => {
          const raw = event.target.value;
          if (definition.valueType === "integer") {
            onChange(
              makeSettingValue("integer", Number.parseInt(raw, 10) || 0),
            );
            return;
          }
          if (definition.valueType === "float") {
            onChange(makeSettingValue("float", Number.parseFloat(raw) || 0));
            return;
          }
          onChange(makeSettingValue("string", raw));
        }}
      />
    </div>
  );
}

function SettingsGroup({
  title,
  description,
  definitions,
  settings,
  readOnly,
  onChange,
}: {
  title: string;
  description: string;
  definitions: SettingDefinition[];
  settings: InstanceSettings;
  readOnly?: boolean;
  onChange: (settings: InstanceSettings) => void;
}) {
  return (
    <div className="space-y-4">
      <div className="space-y-1">
        <h3 className="font-display text-sm tracking-[0.12em] text-accent uppercase">
          {title}
        </h3>
        <p className="font-serif text-sm leading-relaxed text-foreground/65">
          {description}
        </p>
      </div>
      <div className="flex flex-col gap-3">
        {definitions.map((definition) => {
          const value =
            definition.category === "tes3mpGameplay"
              ? getTes3mpValue(
                  settings,
                  definition.key,
                  definition.defaultValue,
                )
              : getOpenMwValue(settings, definition);

          return (
            <SettingField
              key={`${definition.category}-${definition.key}`}
              definition={definition}
              value={value}
              disabled={readOnly}
              onChange={(nextValue) => {
                onChange(
                  definition.category === "tes3mpGameplay"
                    ? setTes3mpValue(settings, definition.key, nextValue)
                    : setOpenMwValue(settings, definition, nextValue),
                );
              }}
            />
          );
        })}
      </div>
    </div>
  );
}

function ManualOverridesSection({
  settings,
  readOnly,
  onChange,
}: {
  settings: InstanceSettings;
  readOnly?: boolean;
  onChange: (settings: InstanceSettings) => void;
}) {
  return (
    <div className="space-y-6">
      <div className="space-y-1">
        <h3 className="font-display text-sm tracking-[0.12em] text-accent uppercase">
          Manual overrides
        </h3>
        <p className="font-serif text-sm leading-relaxed text-foreground/65">
          Add OpenMW keys Nerevar does not expose in the UI. These are saved
          with your instance settings, synced through the manifest, and merged
          into each player&apos;s config at launch.
        </p>
      </div>

      <div className="space-y-2 rounded-lg border border-border/50 bg-background/20 px-4 py-3">
        <Label className="font-display text-sm tracking-[0.08em] text-foreground">
          openmw.cfg lines
        </Label>
        <p className="font-serif text-sm leading-relaxed text-foreground/65">
          One <code className="text-foreground/80">key=value</code> per line,
          appended to the launch overlay and merged into{" "}
          <code className="text-foreground/80">openmw.cfg</code>. Example:{" "}
          <code className="text-foreground/80">groundcover=Mod.esp</code>
        </p>
        <Textarea
          value={settings.openmwCfgOverrides.join("\n")}
          disabled={readOnly}
          className="min-h-32 font-mono text-sm"
          placeholder={"groundcover=Mod.esp\ncontent=Extra.esp"}
          onChange={(event) => {
            onChange({
              ...settings,
              openmwCfgOverrides: event.target.value
                .split("\n")
                .map((line) => line.trim())
                .filter((line) => line.length > 0),
            });
          }}
        />
      </div>

      <div className="space-y-2 rounded-lg border border-border/50 bg-background/20 px-4 py-3">
        <Label className="font-display text-sm tracking-[0.08em] text-foreground">
          settings.cfg text
        </Label>
        <p className="font-serif text-sm leading-relaxed text-foreground/65">
          Raw INI blocks appended to the settings launch overlay. Use standard{" "}
          <code className="text-foreground/80">[Section]</code> headers and{" "}
          <code className="text-foreground/80">key = value</code> lines.
        </p>
        <Textarea
          value={settings.openmwSettingsCfgOverrides}
          disabled={readOnly}
          className="min-h-40 font-mono text-sm"
          placeholder={
            "[Cells]\nviewing distance = 7168\n\n[General]\ncamera sensitivity = 1.0"
          }
          onChange={(event) => {
            onChange({
              ...settings,
              openmwSettingsCfgOverrides: event.target.value,
            });
          }}
        />
      </div>
    </div>
  );
}

export function InstanceServerSettingsManager({
  instanceId,
  instanceName,
  readOnly = false,
}: InstanceServerSettingsManagerProps) {
  const [definitions, setDefinitions] = useState<SettingDefinition[]>([]);
  const [settings, setSettings] = useState<InstanceSettings | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const [defs, current] = await Promise.all([
        invoke<SettingDefinition[]>("get_instance_setting_definitions"),
        invoke<InstanceSettings>("get_instance_settings", { instanceId }),
      ]);
      setDefinitions(defs);
      setSettings(current);
    } catch (error) {
      toast.error(String(error));
    } finally {
      setLoading(false);
    }
  }, [instanceId]);

  useEffect(() => {
    void load();
  }, [load]);

  const grouped = useMemo(
    () => ({
      gameplay: definitions.filter((d) => d.category === "tes3mpGameplay"),
      graphics: definitions.filter((d) => d.category === "openMwGraphics"),
      shaders: definitions.filter((d) => d.category === "openMwShaders"),
    }),
    [definitions],
  );

  const handleSave = async () => {
    if (!settings || readOnly) return;
    setSaving(true);
    try {
      const saved = await invoke<InstanceSettings>(
        "save_instance_settings_command",
        {
          instanceId,
          settings,
        },
      );
      setSettings(saved);
      toast.success("Server settings saved");
    } catch (error) {
      toast.error(String(error));
    } finally {
      setSaving(false);
    }
  };

  if (loading || !settings) {
    return (
      <div className="flex min-h-[240px] items-center justify-center">
        <Loader2 className="size-6 animate-spin text-accent" />
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-4">
      <Card className={INSTANCE_DATA_CARD_CLASS} disableHover disableTap>
        <CardHeader className="border-b border-border/50 px-6 pb-4 pt-6">
          <div className="flex flex-wrap items-start justify-between gap-3">
            <div className="space-y-2">
              <div className="flex items-center gap-2">
                <ServerCog className="size-5 text-accent" />
                <CardTitle className="font-display text-base tracking-[0.15em] text-accent uppercase">
                  Server settings
                </CardTitle>
              </div>
              <CardDescription className="font-serif text-sm leading-relaxed text-foreground/70">
                Configure gameplay rules for {instanceName}. Gameplay settings
                are written to TES3MP <code>config.lua</code> and enforced by
                your server. Graphics and shader settings are synced through the
                manifest and applied to each player&apos;s OpenMW{" "}
                <code>settings.cfg</code> at launch.
              </CardDescription>
            </div>
            {readOnly ? (
              <Badge variant="outline" className="border-accent/30 text-accent">
                Read only
              </Badge>
            ) : null}
          </div>
        </CardHeader>

        <CardContent className="px-6 py-5">
          <Tabs defaultValue="gameplay" className="gap-4">
            <TabsList className="grid h-auto w-full grid-cols-2 gap-1 bg-input/30 p-1 sm:grid-cols-4">
              <TabsTrigger value="gameplay">Gameplay</TabsTrigger>
              <TabsTrigger value="graphics">Graphics</TabsTrigger>
              <TabsTrigger value="shaders">Shaders</TabsTrigger>
              <TabsTrigger value="advanced">Advanced</TabsTrigger>
            </TabsList>

            <TabsContent value="gameplay" className="mt-0 outline-none">
              <SettingsGroup
                title="TES3MP gameplay"
                description="Applied to tes3mp/server/scripts/config.lua and enforced when players connect. Matches OpenMW Game-category settings documented for TES3MP servers."
                definitions={grouped.gameplay}
                settings={settings}
                readOnly={readOnly}
                onChange={setSettings}
              />
            </TabsContent>
            <TabsContent value="graphics" className="mt-0 outline-none">
              <SettingsGroup
                title="OpenMW graphics"
                description="Client graphics settings synced with your manifest and merged into settings.cfg at launch."
                definitions={grouped.graphics}
                settings={settings}
                readOnly={readOnly}
                onChange={setSettings}
              />
            </TabsContent>
            <TabsContent value="shaders" className="mt-0 outline-none">
              <SettingsGroup
                title="OpenMW shaders"
                description="Shader and visual compatibility settings that affect how modded meshes, normal maps, and lighting render for all players."
                definitions={grouped.shaders}
                settings={settings}
                readOnly={readOnly}
                onChange={setSettings}
              />
            </TabsContent>
            <TabsContent value="advanced" className="mt-0 outline-none">
              <ManualOverridesSection
                settings={settings}
                readOnly={readOnly}
                onChange={setSettings}
              />
            </TabsContent>
          </Tabs>
        </CardContent>
      </Card>

      {!readOnly ? (
        <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
          <p className="font-serif text-sm text-foreground/60">
            After saving, use Save &amp; host manifest in the data manager so
            connected players receive these settings.
          </p>
          <Button
            variant="launch"
            className="shrink-0"
            disabled={saving}
            onClick={() => void handleSave()}
          >
            {saving ? (
              <Loader2 className="animate-spin" data-icon="inline-start" />
            ) : (
              <Save data-icon="inline-start" />
            )}
            Save settings
          </Button>
        </div>
      ) : null}
    </div>
  );
}
