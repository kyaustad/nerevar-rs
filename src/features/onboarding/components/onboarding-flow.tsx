import { NerevarHeader } from "@/components/custom/nerevar-header";
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
import { useConfig } from "@/features/config/context/config-context-provider";
import { cn } from "@/lib/utils";
import { invoke } from "@tauri-apps/api/core";
import {
  ArrowLeft,
  ArrowRight,
  Check,
  FolderOpen,
  Layers,
  Network,
  Sparkles,
  Database,
  Download,
  ShieldCheck,
} from "lucide-react";
import { AnimatePresence, motion, useReducedMotion } from "motion/react";
import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";

export type OnboardingStage =
  | "select-data-dir"
  | "select-sync-port"
  | "tutorial"
  | "complete";

const STAGES: { id: OnboardingStage; label: string }[] = [
  { id: "select-data-dir", label: "Data" },
  { id: "select-sync-port", label: "Port" },
  { id: "tutorial", label: "Guide" },
  { id: "complete", label: "Done" },
];

const EASE = [0.22, 1, 0.36, 1] as const;

const slideVariants = {
  enter: (direction: number) => ({
    opacity: 0,
    x: direction > 0 ? 28 : -28,
    filter: "blur(4px)",
  }),
  center: {
    opacity: 1,
    x: 0,
    filter: "blur(0px)",
  },
  exit: (direction: number) => ({
    opacity: 0,
    x: direction > 0 ? -20 : 20,
    filter: "blur(4px)",
  }),
};

type OnboardingFlowProps = {
  stage: OnboardingStage;
  onStageChange: (stage: OnboardingStage) => void;
  onFinish: () => void;
};

export function OnboardingFlow({
  stage,
  onStageChange,
  onFinish,
}: OnboardingFlowProps) {
  const reduceMotion = useReducedMotion();
  const stageIndex = STAGES.findIndex((s) => s.id === stage);
  const direction = 1;

  const goNext = () => {
    const next = STAGES[stageIndex + 1];
    if (next) onStageChange(next.id);
  };

  const goBack = () => {
    const prev = STAGES[stageIndex - 1];
    if (prev) onStageChange(prev.id);
  };

  return (
    <div className="flex h-full w-full flex-col items-center justify-center px-4 py-10">
      <NerevarHeader title="NEREVAR" subtitle="Setup" />

      <OnboardingProgress currentIndex={stageIndex} />

      <div className="relative mt-8 w-full max-w-lg">
        <AnimatePresence mode="wait" custom={direction}>
          <motion.div
            key={stage}
            custom={direction}
            variants={reduceMotion ? undefined : slideVariants}
            initial={reduceMotion ? false : "enter"}
            animate={reduceMotion ? undefined : "center"}
            exit={reduceMotion ? undefined : "exit"}
            transition={{ duration: 0.28, ease: EASE }}
            className="w-full"
          >
            {stage === "select-data-dir" && (
              <SelectDataDirStep onNext={goNext} />
            )}
            {stage === "select-sync-port" && (
              <SelectSyncPortStep onNext={goNext} onBack={goBack} />
            )}
            {stage === "tutorial" && (
              <TutorialStep onNext={goNext} onBack={goBack} />
            )}
            {stage === "complete" && (
              <CompleteStep onFinish={onFinish} onBack={goBack} />
            )}
          </motion.div>
        </AnimatePresence>
      </div>
    </div>
  );
}

function OnboardingProgress({ currentIndex }: { currentIndex: number }) {
  return (
    <ol className="flex items-center gap-2 sm:gap-3">
      {STAGES.map((step, index) => {
        const isComplete = index < currentIndex;
        const isCurrent = index === currentIndex;

        return (
          <li key={step.id} className="flex items-center gap-2 sm:gap-3">
            <motion.div
              layout
              className={cn(
                "flex items-center gap-2 rounded-full border px-2.5 py-1 transition-colors sm:px-3",
                isCurrent &&
                  "border-accent/50 bg-card/80 shadow-[0_0_12px_hsl(var(--accent)/0.15)]",
                isComplete && "border-accent/30 bg-card/50",
                !isCurrent && !isComplete && "border-border/50 bg-card/30",
              )}
              transition={{ duration: 0.25, ease: EASE }}
            >
              <span
                className={cn(
                  "flex size-5 items-center justify-center rounded-full text-[0.75rem] font-display font-semibold",
                  isCurrent && "bg-accent text-accent-foreground",
                  isComplete && "bg-accent/80 text-accent-foreground",
                  !isCurrent && !isComplete && "bg-muted text-muted-foreground",
                )}
              >
                {isComplete ? <Check className="size-3" /> : index + 1}
              </span>
              <span
                className={cn(
                  "hidden font-display text-[0.75rem] tracking-[0.3em] uppercase sm:inline",
                  isCurrent ? "text-accent font-bold" : "text-foreground/55",
                )}
              >
                {step.label}
              </span>
            </motion.div>
            {index < STAGES.length - 1 && (
              <div
                className={cn(
                  "h-px w-4 sm:w-6",
                  index < currentIndex ? "bg-accent/50" : "bg-border/60",
                )}
              />
            )}
          </li>
        );
      })}
    </ol>
  );
}

function OnboardingStepCard({
  children,
  className,
}: {
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <Card
      className={cn(
        "gap-0 border-border/80 bg-card/75 py-0 shadow-[0_0_20px_hsl(var(--accent)/0.08)] ring-accent/15 backdrop-blur-sm",
        className,
      )}
    >
      {children}
    </Card>
  );
}

function StepActions({
  onBack,
  onPrimary,
  primaryLabel,
  showBack = true,
  nextDisabled = false,
}: {
  onBack?: () => void;
  onPrimary: () => void;
  primaryLabel: string;
  showBack?: boolean;
  nextDisabled?: boolean;
}) {
  return (
    <div className="flex items-center justify-between gap-3 border-t border-border/50 px-6 py-4">
      {showBack && onBack ? (
        <Button
          variant="outline"
          size="sm"
          onClick={onBack}
          className="text-foreground/70"
        >
          <ArrowLeft data-icon="inline-start" />
          Back
        </Button>
      ) : (
        <span />
      )}
      <Button
        variant="launch"
        size="sm"
        onClick={onPrimary}
        disabled={nextDisabled}
      >
        {primaryLabel}
        <ArrowRight data-icon="inline-end" />
      </Button>
    </div>
  );
}

function SelectDataDirStep({ onNext }: { onNext: () => void }) {
  const config = useConfig();
  const [dataDir, setDataDir] = useState<string>(config?.rootPath || "");

  const handleBrowse = async () => {
    const path = await invoke<string>("open_directory_picker");
    if (path) {
      setDataDir(path);
    } else {
      toast.error("No directory selected");
    }
  };

  return (
    <OnboardingStepCard>
      <CardHeader className="border-b border-border/50 px-6 pb-4 pt-6">
        <div className="mb-3 flex size-10 items-center justify-center rounded-lg border border-accent/40 bg-accent/10 text-accent">
          <FolderOpen className="size-5" />
        </div>
        <CardTitle className="font-display text-base font-bold tracking-[0.2em] text-accent uppercase">
          Choose data directory
        </CardTitle>
        <CardDescription className="font-serif text-base font-light tracking-[0.05em] leading-relaxed text-foreground/75">
          Pick where Nerevar stores TES3MP, instances, and downloaded mod files.
          This is separate from the app&apos;s own config folder.
        </CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-4 px-6 py-5">
        <div className="space-y-2">
          <Label
            htmlFor="data-dir"
            className="font-display text-[0.75rem] tracking-[0.3em] uppercase text-foreground/70"
          >
            Data directory
          </Label>
          <div className="flex gap-2">
            <Input
              id="data-dir"
              readOnly
              placeholder="C:\Games\Nerevar"
              value={dataDir}
              onChange={(e) => setDataDir(e.target.value)}
              className="font-mono text-xs bg-input/40"
            />
            <Button
              type="button"
              variant="outline"
              className="shrink-0 font-display text-[0.75rem] tracking-[0.3em] uppercase"
              onClick={handleBrowse}
            >
              Browse
            </Button>
          </div>
        </div>
      </CardContent>
      <StepActions
        onPrimary={() => {
          invoke<void>("set_root_path", { path: dataDir })
            .then(() => {
              toast.success("Data directory set");
              onNext();
            })
            .catch((error) => {
              toast.error(`Failed to set data directory: ${error}`);
            });
        }}
        primaryLabel="Continue"
        showBack={false}
        nextDisabled={!dataDir || dataDir.length === 0}
      />
    </OnboardingStepCard>
  );
}

function SelectSyncPortStep({
  onNext,
  onBack,
}: {
  onNext: () => void;
  onBack: () => void;
}) {
  const config = useConfig();
  const [syncPort, setSyncPort] = useState<number>(config?.syncPort || 25567);

  return (
    <OnboardingStepCard>
      <CardHeader className="border-b border-border/50 px-6 pb-4 pt-6">
        <div className="mb-3 flex size-10 items-center justify-center rounded-lg border border-accent/40 bg-accent/10 text-accent">
          <Network className="size-5" />
        </div>
        <CardTitle className="font-display text-base font-bold tracking-[0.2em] text-accent uppercase">
          Sync server port
        </CardTitle>
        <CardDescription className="font-serif text-base font-light tracking-[0.05em] leading-relaxed text-foreground/75">
          The local HTTP server port used for sync and instance coordination.
          Default is 25567 if you&apos;re unsure.
        </CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-4 px-6 py-5">
        <div className="space-y-2">
          <Label
            htmlFor="sync-port"
            className="font-display text-[0.75rem] tracking-[0.3em] uppercase text-foreground/70"
          >
            Port
          </Label>
          <Input
            id="sync-port"
            type="number"
            min={1024}
            max={65535}
            value={syncPort}
            onChange={(e) => setSyncPort(Number(e.target.value))}
            className="font-mono tabular-nums"
          />
        </div>
      </CardContent>
      <StepActions
        onBack={onBack}
        onPrimary={() => {
          invoke<void>("set_sync_port", { port: syncPort })
            .then(() => {
              toast.success("Sync port set");
              onNext();
            })
            .catch((error) => {
              toast.error(`Failed to set sync port: ${error}`);
            });
        }}
        primaryLabel="Continue"
      />
    </OnboardingStepCard>
  );
}

const TUTORIAL_ITEMS = [
  {
    icon: Layers,
    title: "Instances",
    description:
      "Each instance is a separate Morrowind multiplayer setup with its own mods and saves.",
  },
  {
    icon: FolderOpen,
    title: "Isolated data",
    description:
      "Instance files live under your data directory so builds never clash.",
  },
  {
    icon: Database,
    title: "Sync server",
    description:
      "Set an instance as the active instance and clients that connect to your server will use the sync port to sync the servers mods and data so everything stays identical.",
  },
  {
    icon: Download,
    title: "Incremental updates",
    description:
      "This allows you to update your instance and its data and mods incrementally so your players don't have to redownload or manage their data to ensure they can still connect and play.",
  },
  {
    icon: ShieldCheck,
    title: "Data validation",
    description:
      "When a user attempts to connect to your server, Nerevar will validate their data against your instance's manifest to ensure they have the correct files, versions, hashes and load order.",
  },
  {
    icon: Sparkles,
    title: "Launch from Nerevar",
    description:
      "Start client and server together from the dashboard when you're ready.",
  },
] as const;

function TutorialStep({
  onNext,
  onBack,
}: {
  onNext: () => void;
  onBack: () => void;
}) {
  const reduceMotion = useReducedMotion();
  const [hasScrolledToBottom, setHasScrolledToBottom] = useState(false);
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const scrollContainer = scrollRef.current;
    if (!scrollContainer) return;

    const markIfAtBottom = () => {
      const { scrollTop, scrollHeight, clientHeight } = scrollContainer;

      // Content fits without scrolling — nothing else to read.
      if (scrollHeight <= clientHeight + 1) {
        setHasScrolledToBottom(true);
        return;
      }

      setHasScrolledToBottom(scrollTop + clientHeight >= scrollHeight - 8);
    };

    markIfAtBottom();

    scrollContainer.addEventListener("scroll", markIfAtBottom, {
      passive: true,
    });

    const resizeObserver = new ResizeObserver(markIfAtBottom);
    resizeObserver.observe(scrollContainer);

    return () => {
      scrollContainer.removeEventListener("scroll", markIfAtBottom);
      resizeObserver.disconnect();
    };
  }, []);

  return (
    <OnboardingStepCard>
      <CardHeader className="border-b border-border/50 px-6 pb-4 pt-6">
        <CardTitle className="font-display text-base font-bold tracking-[0.2em] text-accent uppercase">
          How instances work
        </CardTitle>
        <CardDescription className="font-serif text-base font-light tracking-[0.05em] leading-relaxed text-foreground/75">
          A quick overview before you enter the manager.
        </CardDescription>
      </CardHeader>
      <CardContent
        ref={scrollRef}
        className="flex max-h-[400px] flex-col gap-3 overflow-y-auto px-6 py-5"
      >
        {TUTORIAL_ITEMS.map((item, index) => (
          <motion.div
            key={item.title}
            initial={reduceMotion ? false : { opacity: 0, y: 10 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: index * 0.07, duration: 0.3, ease: EASE }}
            className="flex gap-3 rounded-lg border border-border/50 bg-background/30 px-4 py-3"
          >
            <div className="flex size-9 shrink-0 items-center justify-center rounded-md border border-accent/30 text-accent/80">
              <item.icon className="size-4" />
            </div>
            <div>
              <p className="font-display text-sm tracking-[0.15em] text-accent uppercase">
                {item.title}
              </p>
              <p className="mt-1 text-base font-light tracking-[0.05em] leading-relaxed text-foreground/70">
                {item.description}
              </p>
            </div>
          </motion.div>
        ))}
        <div className="h-px shrink-0" aria-hidden />
      </CardContent>
      <StepActions
        onBack={onBack}
        onPrimary={onNext}
        primaryLabel="Almost done"
        nextDisabled={!hasScrolledToBottom}
      />
    </OnboardingStepCard>
  );
}

function CompleteStep({
  onFinish,
  onBack,
}: {
  onFinish: () => void;
  onBack: () => void;
}) {
  const reduceMotion = useReducedMotion();

  return (
    <OnboardingStepCard>
      <CardContent className="flex flex-col items-center px-6 py-10 text-center">
        <motion.div
          initial={reduceMotion ? false : { scale: 0.85, opacity: 0 }}
          animate={{ scale: 1, opacity: 1 }}
          transition={{ duration: 0.35, ease: EASE }}
          className="mb-5 flex size-16 items-center justify-center rounded-full border border-accent/40 bg-accent/15 glow-gold animate-pulse transition-all duration-1000 ease-in-out"
        >
          <Check className="size-8 text-accent" strokeWidth={2} />
        </motion.div>
        <h2 className="font-display text-xl tracking-[0.1em] text-gradient-gold">
          {`You're ready`}
        </h2>
        <p className="mt-3 max-w-sm font-serif text-[0.95rem] tracking-[0.05em] font-light leading-relaxed text-foreground/75">
          {`Setup is complete. Welcome to Nerevar!.`}
        </p>

        <Button
          variant="launch"
          size="lg"
          className="mt-8 h-10 px-6"
          onClick={onFinish}
        >
          Enter Nerevar
          <Sparkles data-icon="inline-end" className="size-4" />
        </Button>
        <Button
          variant="outline"
          size="sm"
          className="mt-3 text-foreground/60"
          onClick={onBack}
        >
          <ArrowLeft data-icon="inline-start" />
          Back
        </Button>
      </CardContent>
    </OnboardingStepCard>
  );
}
