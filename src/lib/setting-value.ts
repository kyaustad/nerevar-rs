import type { SettingValue, SettingValueType } from "@/types";

export function displaySettingValue(value: SettingValue): boolean | number | string {
  if ("boolean" in value) return value.boolean;
  if ("integer" in value) return value.integer;
  if ("float" in value) return value.float;
  return value.string;
}

export function makeSettingValue(
  valueType: SettingValueType,
  raw: boolean | number | string,
): SettingValue {
  switch (valueType) {
    case "boolean":
      return { boolean: Boolean(raw) };
    case "integer":
      return { integer: Math.trunc(Number(raw)) || 0 };
    case "float":
      return { float: Number(raw) || 0 };
    case "string":
      return { string: String(raw) };
  }
}
