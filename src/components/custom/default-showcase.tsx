import {
  Calendar,
  ChevronRight,
  Layers,
  Play,
  Plus,
  Scroll,
  Server,
  Settings,
} from "lucide-react";

import { motion, useReducedMotion } from "motion/react";

import { NerevarBackgroundShell } from "@/components/custom/nerevar-background-shell";

import { ThemeSwitcher } from "@/components/custom/theme-switcher";

import { Button } from "@/components/ui/button";

import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";

import { TooltipProvider } from "@/components/ui/tooltip";

import { cn } from "@/lib/utils";

const INSTANCES = [
  {
    name: "VANILLA TES3MP",
    description: "Clean multiplayer experience with no mods installed.",
    mods: 3,
    date: "May 7, 2026",
  },

  {
    name: "ROLEPLAY BUILD",
    description:
      "Immersive RP setup with Tamriel Rebuilt and community patches.",
    mods: 47,
    date: "Mar 15, 2026",
  },
] as const;

function InstanceCard({
  name,
  description,
  mods,
  date,
  index,
}: (typeof INSTANCES)[number] & { index: number }) {
  return (
    <Card
      index={index}
      className="gap-0 border-border/80 bg-card/70 py-0 shadow-[0_0_15px_hsl(var(--accent)/0.08)] ring-accent/20 hover:ring-accent/40 hover:shadow-[0_0_24px_hsl(var(--accent)/0.12)]"
    >
      <CardHeader className="border-b border-border/50 pb-3 pt-4">
        <CardTitle className="font-display text-sm tracking-[0.15em] text-accent uppercase">
          {name}
        </CardTitle>

        <CardAction>
          <Button
            variant="outline"
            size="icon-sm"
            className="text-accent/60 hover:text-accent"
          >
            <ChevronRight />
          </Button>
        </CardAction>

        <CardDescription className="font-serif text-[0.95rem] leading-relaxed text-foreground/75">
          {description}
        </CardDescription>
      </CardHeader>

      <CardContent className="flex flex-col gap-4 pb-4 pt-3">
        <div className="flex flex-wrap gap-4 text-xs tracking-wide text-foreground/65">
          <span className="inline-flex items-center gap-1.5">
            <Layers className="size-3.5 text-accent/70" />
            {mods} mods
          </span>

          <span className="inline-flex items-center gap-1.5">
            <Calendar className="size-3.5 text-accent/70" />

            {date}
          </span>
        </div>

        <div className="grid grid-cols-2 gap-2">
          <Button variant="launch" size="sm" className="h-9 flex-1">
            <Play data-icon="inline-start" />
            Launch Client
          </Button>

          <Button variant="server" size="sm" className="h-9 flex-1">
            <Server data-icon="inline-start" />
            Launch Server
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}

function NewInstanceCard({ index }: { index: number }) {
  return (
    <Card
      index={index}
      className={cn(
        "flex min-h-[220px] cursor-pointer items-center justify-center border-dashed bg-transparent py-0",

        "border-accent/30 shadow-none ring-0 hover:border-accent/50 hover:bg-card/25",
      )}
    >
      <CardContent className="flex flex-col items-center gap-3 py-8">
        <div className="flex size-12 items-center justify-center rounded-lg border border-accent/40 text-accent/70">
          <Plus className="size-6" />
        </div>

        <span className="font-display text-xs tracking-[0.25em] text-foreground/60 uppercase">
          New Instance
        </span>
      </CardContent>
    </Card>
  );
}

function HeroSection() {
  const reduceMotion = useReducedMotion();

  const icon = (
    <div className="mb-5 flex size-16 items-center justify-center rounded-xl bg-gradient-to-br from-accent/80 to-accent/40 glow-gold">
      <Scroll className="size-8 text-accent-foreground" strokeWidth={1.5} />
    </div>
  );

  return (
    <section className="flex flex-col items-center py-12 text-center">
      {reduceMotion ? (
        icon
      ) : (
        <motion.div
          initial={{ opacity: 0, scale: 0.9 }}
          animate={{ opacity: 1, scale: 1 }}
          transition={{ duration: 0.5, ease: [0.22, 1, 0.36, 1] }}
        >
          {icon}
        </motion.div>
      )}

      <h1 className="font-display text-4xl font-bold tracking-[0.12em] text-gradient-gold md:text-5xl">
        NEREVAR
      </h1>

      <p className="mt-2 font-display text-xs tracking-[0.35em] text-foreground/70 uppercase">
        Morrowind Multiplayer Manager
      </p>

      <div className="morrowind-divider mt-6 w-48 max-w-full" />
    </section>
  );
}

function App() {
  return (
    <TooltipProvider>
      <NerevarBackgroundShell>
        <header className="flex items-center justify-between border-b border-border/60 px-6 py-4">
          <div className="flex items-center gap-3">
            <div className="flex size-8 items-center justify-center rounded bg-primary font-display text-sm font-bold text-primary-foreground">
              N
            </div>

            <span className="font-display text-xs tracking-[0.3em] text-accent uppercase">
              Nerevar
            </span>
          </div>

          <div className="flex items-center gap-1">
            <ThemeSwitcher />

            <Button
              variant="outline"
              size="icon"
              className="text-accent hover:text-accent"
            >
              <Settings />

              <span className="sr-only">Settings</span>
            </Button>
          </div>
        </header>

        <main className="mx-auto max-w-5xl px-6 pb-10">
          <HeroSection />

          <section className="grid gap-4 sm:grid-cols-2">
            {INSTANCES.map((instance, i) => (
              <InstanceCard key={instance.name} index={i} {...instance} />
            ))}

            <NewInstanceCard index={INSTANCES.length} />
          </section>

          <p className="mt-8 text-center font-display text-[0.65rem] tracking-[0.3em] text-foreground/55 uppercase">
            {INSTANCES.length} instances configured
          </p>
        </main>
      </NerevarBackgroundShell>
    </TooltipProvider>
  );
}

export default App;
