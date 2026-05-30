import { Button } from "@/components/ui/button";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { useConfig } from "@/features/config/context/config-context-provider";
import { useProcessStatus } from "@/features/instances/context/process-status-context";
import { ProcessConsole } from "@/features/instances/components/process-console";
import { cn } from "@/lib/utils";
import { ExternalLink, Loader2, Play, Server, Square } from "lucide-react";
import { useState } from "react";
import { Link } from "wouter";

type ProcessSideRailProps = {
  role: "client" | "server";
  side: "left" | "right";
};

export function ProcessSideRailGate({ side }: { side: "left" | "right" }) {
  const config = useConfig();
  if (!config?.onboardingComplete) return null;

  return (
    <ProcessSideRail
      role={side === "left" ? "server" : "client"}
      side={side}
    />
  );
}

function ProcessSideRail({ role, side }: ProcessSideRailProps) {
  const { client, server, stop, clear, resolveInstanceName } = useProcessStatus();
  const [open, setOpen] = useState(false);
  const state = role === "client" ? client : server;
  const Icon = role === "client" ? Play : Server;
  const label = role === "client" ? "TES3MP Client" : "TES3MP Server";
  const instanceName = resolveInstanceName(state.instanceId);
  const active = state.running || state.launching;
  const hasSession = Boolean(state.instanceId) || state.lines.length > 0;

  return (
    <>
      <button
        type="button"
        onClick={() => setOpen(true)}
        className={cn(
          "group relative flex w-10 shrink-0 flex-col items-center gap-2 border-border/40 bg-background/60 py-3 text-xs transition-colors hover:bg-background/80",
          side === "left" ? "border-r" : "border-l",
          active && "bg-accent/5",
        )}
        aria-label={`Open ${label} panel`}
      >
        <span
          className={cn(
            "size-2 rounded-full",
            state.running
              ? "bg-emerald-500 animate-pulse"
              : state.launching
                ? "bg-accent animate-pulse"
                : hasSession
                  ? "bg-foreground/35"
                  : "bg-foreground/20",
          )}
        />
        <Icon
          className={cn(
            "size-3.5 shrink-0",
            active ? "text-accent" : "text-foreground/45 group-hover:text-foreground/70",
          )}
        />
        <span
          className={cn(
            "font-display text-[0.6rem] leading-none tracking-[0.18em] uppercase [writing-mode:vertical-rl]",
            active ? "text-foreground/85" : "text-foreground/45",
          )}
        >
          {label}
        </span>
        {active && instanceName ? (
          <span className="max-h-24 truncate font-serif text-[0.58rem] leading-tight text-foreground/60 [writing-mode:vertical-rl]">
            {instanceName}
          </span>
        ) : null}
      </button>

      <Sheet open={open} onOpenChange={setOpen}>
        <SheetContent
          side={side}
          className="flex w-full flex-col gap-0 border-border/60 bg-card/95 p-0 sm:max-w-md"
        >
          <SheetHeader className="border-b border-border/50 px-4 py-4 text-left">
            <SheetTitle className="font-display text-lg tracking-[0.12em] text-gradient-gold uppercase">
              {label}
            </SheetTitle>
            <SheetDescription className="font-serif text-base text-foreground/70">
              {active && instanceName
                ? `Running for ${instanceName}`
                : state.instanceId && instanceName
                  ? `Last session: ${instanceName}`
                  : "No active process"}
            </SheetDescription>
          </SheetHeader>

          <div className="flex flex-wrap gap-2 border-b border-border/40 px-4 py-3">
            {state.instanceId ? (
              <Button variant="outline" size="sm" className="h-9" asChild>
                <Link href={`/instances/${encodeURIComponent(state.instanceId)}`}>
                  <ExternalLink data-icon="inline-start" />
                  View instance
                </Link>
              </Button>
            ) : null}
            <Button
              variant="outline"
              size="sm"
              className="h-9"
              disabled={!state.running}
              onClick={() => void stop(role)}
            >
              <Square data-icon="inline-start" />
              Stop
            </Button>
          </div>

          <div className="min-h-0 flex-1 px-4 py-4">
            <ProcessConsole
              title={`${label} output`}
              lines={state.lines}
              running={state.running}
              onClear={() => clear(role)}
              fillHeight
              emptyHint={
                state.launching
                  ? "Launching…"
                  : "Output appears here while the process runs."
              }
            />
            {state.launching ? (
              <div className="mt-3 flex items-center gap-2 font-serif text-sm text-foreground/65">
                <Loader2 className="size-4 animate-spin text-accent" />
                Starting process…
              </div>
            ) : null}
          </div>
        </SheetContent>
      </Sheet>
    </>
  );
}
