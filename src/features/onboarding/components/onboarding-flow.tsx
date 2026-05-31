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
  Gamepad2,
  XIcon,
  CheckCircleIcon,
} from "lucide-react";
import { AnimatePresence, motion, useReducedMotion } from "motion/react";
import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";

export type OnboardingStage =
  | "select-data-dir"
  | "select-sync-port"
  | "morrowind-installation"
  | "tutorial"
  | "complete";

const STAGES: { id: OnboardingStage; label: string }[] = [
  { id: "select-data-dir", label: "Data" },
  { id: "morrowind-installation", label: "Morrowind" },
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
            {stage === "morrowind-installation" && (
              <MorrowindInstallationStep onNext={goNext} />
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

function MorrowindInstallationStep({ onNext }: { onNext: () => void }) {
  const [openMWConfigValid, setOpenMWConfigValid] = useState<boolean>(false);
  const [hasRunInitialCheck] = useState<boolean>(false);
  const [morrowindInstallationPath, setMorrowindInstallationPath] =
    useState<string>("");
  const reduceMotion = useReducedMotion();

  const handleBrowse = async () => {
    const path = await invoke<string>("open_esm_file_picker");
    if (path) {
      // remove the morrowind.esm from the path to return just the Data Files portion
      if (path.match(/[/\\]Morrowind\.esm$/i)) {
        setMorrowindInstallationPath(path.replace(/[/\\]Morrowind\.esm$/i, ""));
      } else {
        toast.error("Selected file is not the Morrowind.esm file");
      }
    } else {
      toast.error("No file selected");
    }
  };

  const validateOpenMWConfig = async () => {
    const validationResults = await invoke<boolean>(
      "validate_global_openmw_config",
    );
    if (validationResults) {
      setOpenMWConfigValid(true);
    } else {
      setOpenMWConfigValid(false);
    }
  };

  const handleGenerateDefaultOpenMWConfig = async () => {
    invoke<void>("generate_default_global_openmw_config", {
      morrowindInstallationPath,
    })
      .then(() => {
        toast.success("Nerevar OpenMW scaffold created");
        validateOpenMWConfig();
      })
      .catch((error) => {
        toast.error(`Failed to generate default OpenMW config: ${error}`);
      });
  };

  useEffect(() => {
    if (!hasRunInitialCheck) {
      validateOpenMWConfig();
    }
  }, []);

  return (
    <OnboardingStepCard>
      <CardHeader className="border-b border-border/50 px-6 pb-4 pt-6">
        <div className="mb-3 flex size-10 items-center justify-center rounded-lg border border-accent/40 bg-accent/10 text-accent">
          <Gamepad2 className="size-5" />
        </div>
        <CardTitle className="font-display text-base font-bold tracking-[0.2em] text-accent uppercase">
          Locate Morrowind Installation
        </CardTitle>
        <CardDescription className="font-serif text-base font-light tracking-[0.05em] leading-relaxed text-foreground/75">
          Nerevar keeps your personal{" "}
          <code className="font-mono text-sm text-accent bg-secondary/70 p-1 whitespace-nowrap">
            openmw.cfg
          </code>{" "}
          untouched. We back it up to{" "}
          <code className="font-mono text-sm text-accent bg-secondary/70 p-1 whitespace-nowrap">
            openmw.backup.cfg
          </code>{" "}
          and create{" "}
          <code className="font-mono text-sm text-accent bg-secondary/70 p-1 whitespace-nowrap">
            openmw.nerevar.cfg
          </code>{" "}
          as the base config used when launching TES3MP through Nerevar. A{" "}
          <code className="font-mono text-sm text-accent bg-secondary/70 p-1 whitespace-nowrap">
            Morrowind.ini
          </code>{" "}
          file is not required if you have never launched the game.
        </CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-4 px-6 py-5">
        {openMWConfigValid ? (
          <div className="flex flex-col items-center gap-8">
            <div className="flex flex-row items-center w-full justify-center gap-4 rounded-lg border border-border/50 bg-background/30 p-4">
              <motion.div
                initial={reduceMotion ? false : { scale: 0.85, opacity: 0 }}
                animate={{ scale: 1, opacity: 1 }}
                transition={{ duration: 0.35, ease: EASE }}
                className="mb-5 flex size-16 items-center justify-center rounded-full border border-accent/40 bg-primary/15 text-destructive animate-pulse transition-all duration-1000 ease-in-out"
              >
                <CheckCircleIcon
                  className="size-8 text-emerald-500"
                  strokeWidth={2}
                />
              </motion.div>
              <p className="text-sm text-accent max-w-xs text-center">
                Nerevar OpenMW scaffold is ready. Your existing global config was preserved in{" "}
                <code className="font-mono text-xs">openmw.backup.cfg</code> if one existed.
              </p>
            </div>
          </div>
        ) : (
          <div className="flex flex-col items-center gap-8">
            <div className="flex flex-row items-center w-full justify-center gap-4 rounded-lg border border-border/50 bg-background/30 p-4">
              <motion.div
                initial={reduceMotion ? false : { scale: 0.85, opacity: 0 }}
                animate={{ scale: 1, opacity: 1 }}
                transition={{ duration: 0.35, ease: EASE }}
                className="mb-5 flex size-16 items-center justify-center rounded-full border border-accent/40 bg-primary/15 text-destructive animate-pulse transition-all duration-1000 ease-in-out"
              >
                <XIcon className="size-8 text-destructive" strokeWidth={2} />
              </motion.div>
              <p className="text-sm text-accent max-w-xs text-center">
                Select your Morrowind.esm file so Nerevar can create{" "}
                <code className="font-mono text-xs">openmw.nerevar.cfg</code>.
              </p>
            </div>
            <Label
              htmlFor="morrowind-installation-path"
              className="text-xl font-display text-foreground"
            >
              Find your Morrowind.esm file for Nerevar
            </Label>
            <div className="flex gap-2 w-full">
              <Input
                id="morrowind-installation-path"
                readOnly
                placeholder="C:\Program Files (x86)\Steam\steamapps\common\Morrowind\Data Files"
                value={morrowindInstallationPath}
                className="font-mono text-xs bg-input/40 truncate"
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
            <Button
              type="button"
              variant="launch"
              className="shrink-0 h-10 w-full font-display text-[0.75rem] tracking-[0.3em] uppercase hover:disabled:cursor-not-allowed"
              disabled={
                !morrowindInstallationPath ||
                morrowindInstallationPath.length === 0
              }
              onClick={handleGenerateDefaultOpenMWConfig}
            >
              Generate Nerevar OpenMW scaffold
            </Button>
          </div>
        )}
      </CardContent>
      <StepActions
        onPrimary={onNext}
        primaryLabel="Continue"
        showBack={false}
        nextDisabled={!openMWConfigValid}
      />
    </OnboardingStepCard>
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
