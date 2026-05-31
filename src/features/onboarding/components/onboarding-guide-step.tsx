import {
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { cn } from "@/lib/utils";
import {
  Download,
  FolderSync,
  Gamepad2,
  KeyRound,
  Layers,
  Network,
  Play,
  RefreshCw,
  Server,
  Settings2,
  ShieldCheck,
  Check,
  type LucideIcon,
} from "lucide-react";
import { motion, useReducedMotion } from "motion/react";
import { useEffect, useRef, useState } from "react";
import {
  OnboardingStepCard,
  StepActions,
} from "@/features/onboarding/components/onboarding-step-shell";

const EASE = [0.22, 1, 0.36, 1] as const;

type Persona = "host" | "client";

type PersonaProgress = Record<Persona, boolean>;

const INITIAL_PERSONA_PROGRESS: PersonaProgress = {
  host: false,
  client: false,
};

function scrollHint(progress: PersonaProgress, active: Persona): string {
  const remaining: Persona[] = [];
  if (!progress.host) remaining.push("host");
  if (!progress.client) remaining.push("client");

  if (remaining.length === 2) {
    return active === "host"
      ? "Scroll to the bottom to finish the Hosting guide, then read Joining."
      : "Scroll to the bottom on each tab — start with Hosting or Joining.";
  }

  if (!progress.host) {
    return active === "host"
      ? "Scroll to the bottom to finish the Hosting guide."
      : "Switch to Hosting and scroll to the bottom.";
  }

  return active === "client"
    ? "Scroll to the bottom to finish the Joining guide."
    : "Switch to Joining and scroll to the bottom.";
}

type GuideStep = {
  title: string;
  body: string;
};

type GuideSectionData = {
  id: string;
  label: string;
  icon: LucideIcon;
  intro: string;
  steps: GuideStep[];
};

const SYNC_OVERVIEW = [
  {
    icon: Layers,
    title: "Manifest",
    description:
      "The host builds a manifest — a list of every mod folder, file hash, and plugin load order that players must match.",
  },
  {
    icon: Network,
    title: "Sync server",
    description:
      "Nerevar runs a local sync server on the port you chose. It shares the manifest and mod files with connected players.",
  },
  {
    icon: Download,
    title: "Download & validate",
    description:
      "Clients pull only what changed, then Nerevar verifies checksums and load order before you can launch.",
  },
  {
    icon: ShieldCheck,
    title: "Play together",
    description:
      "Once synced, everyone launches TES3MP through Nerevar with the same data — no manual mod list wrangling.",
  },
] as const;

const HOST_GUIDE: GuideSectionData = {
  id: "host",
  label: "Hosting",
  icon: Server,
  intro:
    "Owned instances are full setups you control. You manage mods, host the sync server, and run the TES3MP game server.",
  steps: [
    {
      title: "Create an owned instance",
      body: "From the dashboard, open Owned Instances → New Instance. Set your server name, TES3MP game port, and optional password.",
    },
    {
      title: "Build your mod list",
      body: "Open the instance data manager to scan folders, set load order, and import from MO2 CSV if you use Mod Organizer 2 (see the MO2 Plugin card on the dashboard).",
    },
    {
      title: "Save & host your manifest",
      body: "When your list is ready, use Save & host manifest in the data manager. This writes the manifest and activates Nerevar sync for that instance.",
    },
    {
      title: "Launch and share connection info",
      body: "Launch the TES3MP server from the instance page. Give players your public IP, Nerevar sync port, and password if you set one. Nerevar handles passing along the port for your TES3MP server to the connecting client. Just make sure port forwarding is setup for both the Nerevar sync port and the TES3MP Server port.",
    },
    {
      title: "Update anytime",
      body: "After changing mods, save & host again. Connected players sync the delta on their next launch or manual sync — no full redownload.",
    },
  ],
};

const CLIENT_GUIDE: GuideSectionData = {
  id: "client",
  label: "Joining",
  icon: Play,
  intro:
    "Synced instances are saved connections to someone else's host. Nerevar keeps your local copy of their mods up to date.",
  steps: [
    {
      title: "Create a new connection",
      body: "From the dashboard, open Synced Instances → New Connection. Enter the host's address, Nerevar sync port, and sync password if their server requires one.",
    },
    {
      title: "Initial sync",
      body: "Nerevar downloads the host manifest and mod files into a local instance folder. This can take a while on first connect.",
    },
    {
      title: "Launch the client",
      body: "Use Launch client on the instance page. Nerevar checks for updates, syncs if needed, then starts TES3MP with the correct load order.",
    },
    {
      title: "Stay up to date",
      body: "When the host updates mods, use Sync from host on the instance page or just launch — Nerevar pulls changes automatically before you connect.",
    },
    {
      title: "Edit connection details",
      body: "Instance settings let you update the host address, sync port, or password without recreating the connection.",
    },
  ],
};

function OverviewCards({ reduceMotion }: { reduceMotion: boolean | null }) {
  return (
    <section className="space-y-3">
      <GuideSectionHeading
        icon={FolderSync}
        title="How syncing works"
        description="The same flow for every server — whether you host or join."
      />
      <div className="grid gap-2">
        {SYNC_OVERVIEW.map((item, index) => (
          <GuideCard
            key={item.title}
            icon={item.icon}
            title={item.title}
            description={item.description}
            index={index}
            reduceMotion={reduceMotion}
          />
        ))}
      </div>
    </section>
  );
}

function PersonaGuide({
  guide,
  reduceMotion,
  baseDelay = 0,
}: {
  guide: GuideSectionData;
  reduceMotion: boolean | null;
  baseDelay?: number;
}) {
  const Icon = guide.icon;

  return (
    <div className="space-y-3">
      <GuideSectionHeading
        icon={Icon}
        title={guide.label}
        description={guide.intro}
      />
      <ol className="flex flex-col gap-2">
        {guide.steps.map((step, index) => (
          <motion.li
            key={step.title}
            initial={reduceMotion ? false : { opacity: 0, y: 8 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{
              delay: baseDelay + index * 0.05,
              duration: 0.28,
              ease: EASE,
            }}
            className="flex gap-3 rounded-lg border border-border/50 bg-background/30 px-3 py-3 text-left"
          >
            <span className="flex size-7 shrink-0 items-center justify-center rounded-full border border-accent/40 font-display text-xs text-accent">
              {index + 1}
            </span>
            <div className="space-y-1">
              <p className="font-display text-sm tracking-[0.08em] text-foreground">
                {step.title}
              </p>
              <p className="font-serif text-sm leading-relaxed text-foreground/70">
                {step.body}
              </p>
            </div>
          </motion.li>
        ))}
      </ol>
    </div>
  );
}

function GuideSectionHeading({
  icon: Icon,
  title,
  description,
}: {
  icon: LucideIcon;
  title: string;
  description: string;
}) {
  return (
    <div className="space-y-1">
      <div className="flex items-center gap-2">
        <Icon className="size-4 text-accent" />
        <h3 className="font-display text-sm tracking-[0.15em] text-accent uppercase">
          {title}
        </h3>
      </div>
      <p className="font-serif text-sm leading-relaxed text-foreground/70">
        {description}
      </p>
    </div>
  );
}

function GuideCard({
  icon: Icon,
  title,
  description,
  index,
  reduceMotion,
}: {
  icon: LucideIcon;
  title: string;
  description: string;
  index: number;
  reduceMotion: boolean | null;
}) {
  return (
    <motion.div
      initial={reduceMotion ? false : { opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ delay: index * 0.05, duration: 0.28, ease: EASE }}
      className="flex gap-3 rounded-lg border border-border/50 bg-background/30 px-3 py-3"
    >
      <div className="flex size-8 shrink-0 items-center justify-center rounded-md border border-accent/30 text-accent/80">
        <Icon className="size-4" />
      </div>
      <div>
        <p className="font-display text-sm tracking-[0.12em] text-accent uppercase">
          {title}
        </p>
        <p className="mt-1 font-serif text-sm leading-relaxed text-foreground/70">
          {description}
        </p>
      </div>
    </motion.div>
  );
}

function QuickReference() {
  const items = [
    {
      icon: Settings2,
      label: "Nerevar Settings",
      hint: "Change your sync port anytime.",
    },
    {
      icon: KeyRound,
      label: "Passwords",
      hint: "Host server password doubles as the sync password for clients.",
    },
    {
      icon: RefreshCw,
      label: "Status bar",
      hint: "The top bar shows sync server and hosting status while you play.",
    },
    {
      icon: Gamepad2,
      label: "Both roles",
      hint: "You can host one instance and sync to friends on another — pick the path that fits each server.",
    },
  ];

  return (
    <section className="space-y-2 rounded-lg border border-accent/20 bg-accent/5 px-3 py-3">
      <p className="font-display text-sm tracking-[0.12em] text-accent uppercase">
        Good to know
      </p>
      <ul className="space-y-2">
        {items.map((item) => (
          <li
            key={item.label}
            className="flex gap-2 font-serif text-sm text-foreground/70"
          >
            <item.icon className="mt-0.5 size-4 shrink-0 text-accent/70" />
            <span>
              <span className="text-foreground">{item.label}</span> —{" "}
              {item.hint}
            </span>
          </li>
        ))}
      </ul>
    </section>
  );
}

export function OnboardingGuideStep({
  onNext,
  onBack,
}: {
  onNext: () => void;
  onBack: () => void;
}) {
  const reduceMotion = useReducedMotion();
  const [personaProgress, setPersonaProgress] = useState<PersonaProgress>(
    INITIAL_PERSONA_PROGRESS,
  );
  const [persona, setPersona] = useState<Persona>("host");
  const scrollRef = useRef<HTMLDivElement>(null);

  const canContinue = personaProgress.host && personaProgress.client;

  useEffect(() => {
    const scrollContainer = scrollRef.current;
    if (!scrollContainer) return;

    scrollContainer.scrollTop = 0;

    const markIfAtBottom = () => {
      const { scrollTop, scrollHeight, clientHeight } = scrollContainer;
      const atBottom =
        scrollHeight <= clientHeight + 1 ||
        scrollTop + clientHeight >= scrollHeight - 8;

      if (atBottom) {
        setPersonaProgress((prev) =>
          prev[persona] ? prev : { ...prev, [persona]: true },
        );
      }
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
  }, [persona]);

  return (
    <OnboardingStepCard>
      <CardHeader className="border-b border-border/50 px-6 pb-4 pt-6">
        <CardTitle className="font-display text-base font-bold tracking-[0.2em] text-accent uppercase">
          Your guide to Nerevar
        </CardTitle>
        <CardDescription className="font-serif text-base font-light tracking-[0.05em] leading-relaxed text-foreground/75">
          Learn how mod sync fits together, then follow the path that matches
          how you play — hosting, joining, or both.
        </CardDescription>
      </CardHeader>

      <Tabs
        value={persona}
        onValueChange={(value) => setPersona(value as Persona)}
        className="flex flex-col gap-0"
      >
        <div className="border-b border-border/40 px-6 py-4">
          <TabsList className="grid h-auto w-full grid-cols-2 gap-1 bg-input/30 p-1 group-data-horizontal/tabs:h-auto">
            <TabsTrigger
              value="host"
              className="inline-flex h-9 w-full items-center justify-center gap-2 font-display text-xs tracking-[0.12em] uppercase sm:text-sm"
            >
              <Server className="size-4 shrink-0" />
              Hosting
              {personaProgress.host ? (
                <Check
                  className="size-3.5 shrink-0 text-accent"
                  aria-label="Hosting guide read"
                />
              ) : null}
            </TabsTrigger>
            <TabsTrigger
              value="client"
              className="inline-flex h-9 w-full items-center justify-center gap-2 font-display text-xs tracking-[0.12em] uppercase sm:text-sm"
            >
              <Play className="size-4 shrink-0" />
              Joining
              {personaProgress.client ? (
                <Check
                  className="size-3.5 shrink-0 text-accent"
                  aria-label="Joining guide read"
                />
              ) : null}
            </TabsTrigger>
          </TabsList>
        </div>

        <CardContent
          ref={scrollRef}
          className={cn(
            "flex max-h-[min(52vh,420px)] flex-col gap-5 overflow-y-auto px-6 py-5",
          )}
        >
          <OverviewCards reduceMotion={reduceMotion} />

          <TabsContent value="host" className="mt-0 outline-none">
            <PersonaGuide
              guide={HOST_GUIDE}
              reduceMotion={reduceMotion}
              baseDelay={0.15}
            />
          </TabsContent>
          <TabsContent value="client" className="mt-0 outline-none">
            <PersonaGuide
              guide={CLIENT_GUIDE}
              reduceMotion={reduceMotion}
              baseDelay={0.15}
            />
          </TabsContent>

          <QuickReference />
          <div className="h-px shrink-0" aria-hidden />
        </CardContent>
      </Tabs>

      {!canContinue ? (
        <p className="px-6 pb-2 text-center font-serif text-xs text-foreground/55">
          {scrollHint(personaProgress, persona)}
        </p>
      ) : null}

      <StepActions
        onBack={onBack}
        onPrimary={onNext}
        primaryLabel="Almost done"
        nextDisabled={!canContinue}
      />
    </OnboardingStepCard>
  );
}
