import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";
import type { ProcessLine } from "@/features/instances/context/process-status-context";
import { Trash2 } from "lucide-react";
import { useEffect, useRef } from "react";

type ProcessConsoleProps = {
  title: string;
  lines: ProcessLine[];
  running: boolean;
  onClear: () => void;
  className?: string;
  fillHeight?: boolean;
  emptyHint?: string;
};

export function ProcessConsole({
  title,
  lines,
  running,
  onClear,
  className,
  fillHeight = false,
  emptyHint = "No output yet.",
}: ProcessConsoleProps) {
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (lines.length === 0) return;
    const viewport = bottomRef.current?.closest(
      '[data-slot="scroll-area-viewport"]',
    ) as HTMLElement | null;
    if (viewport) {
      viewport.scrollTop = viewport.scrollHeight;
    }
  }, [lines]);

  return (
    <div
      className={cn(
        "flex flex-col gap-2 rounded-lg border border-border/50 bg-[#0d0b09] p-3",
        fillHeight && "h-full",
        className,
      )}
    >
      <div className="flex items-center justify-between gap-2">
        <div className="flex items-center gap-2">
          <span className="font-display text-sm tracking-[0.15em] text-accent uppercase">
            {title}
          </span>
          <span
            className={cn(
              "size-2 rounded-full",
              running ? "bg-emerald-500 animate-pulse" : "bg-foreground/30",
            )}
          />
        </div>
        <Button
          variant="outline"
          size="icon-xs"
          onClick={onClear}
          disabled={lines.length === 0}
        >
          <Trash2 />
        </Button>
      </div>

      <ScrollArea
        className={cn(
          "w-full rounded-md border border-border/30 bg-black/40",
          fillHeight ? "min-h-0 flex-1" : "h-56",
        )}
      >
        <div className="space-y-0.5 p-3 font-mono text-sm leading-relaxed">
          {lines.length === 0 ? (
            <p className="text-foreground/40">{emptyHint}</p>
          ) : (
            lines.map((line) => (
              <div
                key={line.id}
                className={cn(
                  "whitespace-pre-wrap break-all",
                  line.stream === "stderr"
                    ? "text-amber-200/90"
                    : "text-emerald-100/85",
                )}
              >
                {line.text}
              </div>
            ))
          )}
          <div ref={bottomRef} />
        </div>
      </ScrollArea>
    </div>
  );
}
