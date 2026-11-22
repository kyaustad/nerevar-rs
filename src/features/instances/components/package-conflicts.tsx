import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";
import type { ConflictWith, LoadOrderEntry, PackageConflicts } from "@/types";
import { Zap } from "lucide-react";
import { createContext, useContext } from "react";

type ConflictsContextValue = {
  byEntry: Map<string, PackageConflicts>;
  onShow: (entry: LoadOrderEntry) => void;
};

export const ConflictsContext = createContext<ConflictsContextValue>({
  byEntry: new Map(),
  onShow: () => {},
});

/**
 * MO2-style conflict icon: green when the package only overwrites files of earlier packages, red when later
 * packages overwrite some of its files, yellow for both.
 */
export function ConflictIndicator({ entry }: { entry: LoadOrderEntry }) {
  const { byEntry, onShow } = useContext(ConflictsContext);
  const conflicts = byEntry.get(entry.id);
  if (!conflicts) {
    return <span className="hidden w-8 shrink-0 sm:block" aria-hidden />;
  }
  const wins = conflicts.overwrites.length > 0;
  const loses = conflicts.overwrittenBy.length > 0;
  const label =
    wins && loses
      ? "Overwrites files of other packages, and some of its files are overwritten"
      : wins
        ? "Overwrites files of earlier packages"
        : "Some of its files are overwritten by later packages";
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button
          type="button"
          variant="ghost"
          size="icon-sm"
          aria-label={`File conflicts of ${entry.name}`}
          onClick={() => onShow(entry)}
          className={cn(
            "shrink-0",
            wins && loses
              ? "text-amber-400"
              : wins
                ? "text-emerald-400"
                : "text-red-400",
          )}
        >
          <Zap />
        </Button>
      </TooltipTrigger>
      <TooltipContent>{label}</TooltipContent>
    </Tooltip>
  );
}

function ConflictList({
  title,
  items,
}: {
  title: string;
  items: ConflictWith[];
}) {
  if (items.length === 0) return null;
  return (
    <section className="space-y-2">
      <h3 className="font-display text-sm tracking-[0.12em] text-accent uppercase">
        {title}
      </h3>
      <ul className="space-y-2">
        {items.map((item) => (
          <li
            key={item.entryId}
            className="rounded-md border border-border/40 bg-card/40 px-3 py-2"
          >
            <p className="font-display text-foreground">
              {item.name}{" "}
              <span className="font-serif text-sm text-foreground/55">
                ({item.files} {item.files === 1 ? "file" : "files"})
              </span>
            </p>
            <ul className="mt-1 space-y-0.5 font-mono text-xs text-foreground/60">
              {item.examples.map((file) => (
                <li key={file} className="truncate">
                  {file}
                </li>
              ))}
              {item.files > item.examples.length ? (
                <li>… and {item.files - item.examples.length} more</li>
              ) : null}
            </ul>
          </li>
        ))}
      </ul>
    </section>
  );
}

export function ConflictsDialog({
  entry,
  conflicts,
  onOpenChange,
}: {
  entry: LoadOrderEntry | null;
  conflicts: PackageConflicts | undefined;
  onOpenChange: (open: boolean) => void;
}) {
  return (
    <Dialog open={entry !== null} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle className="font-display tracking-[0.08em]">
            {entry?.name}: file conflicts
          </DialogTitle>
          <DialogDescription className="font-serif text-left">
            OpenMW takes each file from the last package in the load order that
            has it.{" "}
            {conflicts
              ? `${conflicts.winningFiles} of this package's files are used, ${conflicts.losingFiles} are overwritten.`
              : null}
          </DialogDescription>
        </DialogHeader>
        {conflicts ? (
          <div className="max-h-[60vh] space-y-4 overflow-y-auto pr-1">
            <ConflictList
              title="Overwrites files of"
              items={conflicts.overwrites}
            />
            <ConflictList
              title="Files overwritten by"
              items={conflicts.overwrittenBy}
            />
          </div>
        ) : null}
      </DialogContent>
    </Dialog>
  );
}
