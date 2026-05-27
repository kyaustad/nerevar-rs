import re
from pathlib import Path

# Substitute the path to the importer.cpp file with the path to the importer.cpp file in the OpenMW source code
text = Path(r"c:\Morrowind\openmw\apps\mwiniimporter\importer.cpp").read_text(
    encoding="utf-8", errors="replace"
)
start = text.index("const char *fallback[] = {")
end = text.index("};", start) + 2
block = text[start:end]
keys = re.findall(r'"([^"]+)"', block)
keys = [k for k in keys if ":" in k]
out = Path(__file__).resolve().parents[1] / "src-tauri/src/openmw_ini_importer/fallback_keys.rs"
out.parent.mkdir(parents=True, exist_ok=True)
lines = [
    "// Auto-generated from OpenMW apps/mwiniimporter/importer.cpp fallback[]",
    "",
    "pub const FALLBACK_INI_KEYS: &[&str] = &[",
]
for k in keys:
    lines.append(f'    "{k}",')
lines.append("];")
lines.append("")
out.write_text("\n".join(lines), encoding="utf-8")
print(f"Wrote {len(keys)} keys to {out}")
