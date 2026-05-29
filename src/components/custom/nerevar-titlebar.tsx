import { cn } from "@/lib/utils";
import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Minus, Square, X } from "lucide-react";
import { ThemeSwitcher } from "./theme-switcher";
import { SyncHostStatusBar } from "./sync-host-status-bar";
import { TooltipProvider } from "../ui/tooltip";

export function NerevarTitlebarLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <div className="flex h-dvh flex-col overflow-hidden">
      <NerevarTitlebar />
      <SyncHostStatusBar />
      <div className="min-h-0 flex-1 overflow-y-auto">{children}</div>
    </div>
  );
}

function NerevarTitlebar() {
  const win = () => getCurrentWindow();

  return (
    <TooltipProvider>
      <header className="flex h-12 shrink-0 items-center justify-between border-b border-border/60 bg-card/80 px-3 backdrop-blur-sm">
        <div
          data-tauri-drag-region
          className="flex min-w-0 flex-1 items-center gap-2"
        >
          <span className="flex size-6 items-center justify-center rounded bg-primary font-display text-[0.65rem] font-bold text-primary-foreground">
            N
          </span>
          <span className="truncate font-display text-[0.85rem] tracking-[0.2em] font-semibold text-accent uppercase">
            Nerevar
          </span>
        </div>

        <ThemeSwitcher className="mr-8" />

        <div className="flex shrink-0 items-center">
          <TitlebarButton
            label="Minimize"
            onClick={() => isTauri() && win().minimize()}
          >
            <Minus className="size-4" />
          </TitlebarButton>
          <TitlebarButton
            label="Maximize"
            onClick={() => isTauri() && win().toggleMaximize()}
          >
            <Square className="size-3.5" />
          </TitlebarButton>
          <TitlebarButton
            label="Close"
            onClick={() => isTauri() && win().close()}
            close
          >
            <X className="size-4" />
          </TitlebarButton>
        </div>
      </header>
    </TooltipProvider>
  );
}

function TitlebarButton({
  children,
  label,
  onClick,
  close,
}: {
  children: React.ReactNode;
  label: string;
  onClick: () => void;
  close?: boolean;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      onClick={onClick}
      className={cn(
        "inline-flex size-8 items-center justify-center rounded-md text-foreground/60 hover:bg-accent/15 hover:text-accent",
        close && "hover:bg-destructive/20 hover:text-destructive",
      )}
    >
      {children}
    </button>
  );
}
