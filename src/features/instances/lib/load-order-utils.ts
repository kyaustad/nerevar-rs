import type { LoadOrderEntry } from "@/types";

function priorityValue(entry: LoadOrderEntry): number {
  const value = Number(entry.priority);
  return Number.isFinite(value) ? value : 0;
}

export function sortLoadOrderEntries(entries: LoadOrderEntry[]) {
  return [...entries].sort(
    (a, b) => priorityValue(a) - priorityValue(b),
  );
}

/** Assign priorities from the given visual order (index * 10 + 10). */
export function applyOrderToEntries(
  orderedEntries: LoadOrderEntry[],
): LoadOrderEntry[] {
  return orderedEntries.map((entry, index) => ({
    ...entry,
    priority: (index + 1) * 10,
  }));
}

export function normalizePriorities(entries: LoadOrderEntry[]): LoadOrderEntry[] {
  return applyOrderToEntries(sortLoadOrderEntries(entries));
}

export function reorderLoadOrderEntries(
  entries: LoadOrderEntry[],
  fromIndex: number,
  toIndex: number,
): LoadOrderEntry[] {
  const sorted = sortLoadOrderEntries(entries);
  if (
    fromIndex < 0 ||
    toIndex < 0 ||
    fromIndex >= sorted.length ||
    toIndex >= sorted.length ||
    fromIndex === toIndex
  ) {
    return entries.map((entry) => ({ ...entry }));
  }

  const next = [...sorted];
  const [moved] = next.splice(fromIndex, 1);
  next.splice(toIndex, 0, moved);
  return applyOrderToEntries(next);
}

export function moveEntryByOffset(
  entries: LoadOrderEntry[],
  index: number,
  direction: -1 | 1,
): LoadOrderEntry[] {
  return reorderLoadOrderEntries(entries, index, index + direction);
}

export function setEntryPriority(
  entries: LoadOrderEntry[],
  entryId: string,
  rawPriority: string,
): LoadOrderEntry[] {
  const parsed = Number.parseInt(rawPriority, 10);
  if (Number.isNaN(parsed) || parsed < 1) {
    return entries.map((entry) => ({ ...entry }));
  }

  const updated = entries.map((entry) =>
    entry.id === entryId ? { ...entry, priority: parsed } : entry,
  );
  return normalizePriorities(updated);
}
