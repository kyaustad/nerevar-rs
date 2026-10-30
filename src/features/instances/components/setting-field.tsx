import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { displaySettingValue, makeSettingValue } from "@/lib/setting-value";
import type {
  InstanceSettings,
  SettingDefinition,
  SettingValue,
} from "@/types";

/** The settings.cfg section of an OpenMW setting. */
export function openMwSection(definition: SettingDefinition): string {
  switch (definition.category) {
    case "openMwGraphics":
      return "Graphics";
    case "openMwShaders":
      return "Shaders";
    case "openMwPostProcessing":
      return "Post Processing";
    default:
      return "Game";
  }
}

export function getOpenMwValue(
  settings: InstanceSettings,
  definition: SettingDefinition,
): SettingValue {
  const section = openMwSection(definition);
  return (
    settings.openmwSettings[section]?.[definition.key] ??
    definition.defaultValue
  );
}

export function setOpenMwValue(
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

export function SettingField({
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
