import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { cn } from "@/lib/utils";
import { invoke } from "@tauri-apps/api/core";
import {
  ArrowLeft,
  Download,
  FolderOpen,
  ListOrdered,
  Loader2,
  Puzzle,
} from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";
import { Link } from "wouter";

const cardClass =
  "gap-0 border-border/80 bg-card/70 py-0 shadow-[0_0_15px_hsl(var(--accent)/0.08)] ring-1 ring-accent/20";

const steps = [
  {
    title: "Use your MO2 mods folder as instance data",
    body: "When creating an owned instance, point the data directory at your Mod Organizer 2 instance mods folder — the directory that contains one folder per mod.",
  },
  {
    title: "Install this plugin into MO2",
    body: "Copy the Nerevar export plugin into MO2's plugins folder so it appears under Tools in Mod Organizer.",
  },
  {
    title: "Export from MO2",
    body: "In MO2, open Tools → Export Enabled Mods CSV for Nerevar and save a CSV file.",
  },
  {
    title: "Import in Nerevar",
    body: "Open your instance data manager and use Import MO2 CSV to apply mod priorities and plugin load order.",
  },
];

export function Mo2PluginPage() {
  const [installing, setInstalling] = useState(false);

  const installPlugin = async () => {
    setInstalling(true);
    try {
      const installedPath = await invoke<string>("install_mo2_export_plugin", {
        pluginsDirectory: null,
      });
      toast.success(`MO2 plugin installed to ${installedPath}`);
    } catch (error) {
      const message = String(error);
      if (!message.toLowerCase().includes("no directory selected")) {
        toast.error(message);
      }
    } finally {
      setInstalling(false);
    }
  };

  return (
    <div className="mx-auto flex w-full max-w-2xl flex-col gap-5 px-1 pb-10">
      <Button
        variant="outline"
        size="sm"
        className="w-fit font-display text-sm tracking-[0.2em] text-foreground/70 uppercase hover:text-accent"
        asChild
      >
        <Link href="/">
          <ArrowLeft data-icon="inline-start" />
          Dashboard
        </Link>
      </Button>

      <Card className={cardClass}>
        <CardHeader className="border-b border-border/50 space-y-3 pb-4 pt-5">
          <div className="flex items-center gap-2 text-accent">
            <Puzzle className="size-5" />
            <span className="font-display text-[0.7rem] tracking-[0.2em] uppercase">
              Mod Organizer 2
            </span>
          </div>
          <CardTitle className="font-display text-3xl tracking-[0.08em] text-gradient-gold">
            Nerevar MO2 export plugin
          </CardTitle>
          <CardDescription className="font-serif text-left text-[1rem] leading-relaxed text-foreground/75">
            For players who manage TES3MP mods in Mod Organizer 2 and want to
            use an MO2 instance mods directory as a Nerevar instance data
            folder.
          </CardDescription>
        </CardHeader>

        <CardContent className="flex flex-col gap-6 py-5">
          <section className="space-y-3">
            <h2 className="font-display text-lg tracking-[0.15em] text-accent uppercase">
              What it does
            </h2>
            <p className="font-serif text-base leading-relaxed text-foreground/75">
              The plugin exports your enabled MO2 mods and plugins to a CSV file
              that Nerevar understands. That CSV includes each mod&apos;s folder
              name on disk, MO2 priority, plugin names, enabled state, and load
              order — the same fields used by{" "}
              <span className="font-mono text-sm">Import MO2 CSV</span> in the
              instance data manager.
            </p>
            <p className="font-serif text-base leading-relaxed text-foreground/75">
              This workflow is for setups where your mod folders already live
              under an MO2 instance and you do not want to duplicate them into
              a separate Nerevar-managed data tree.
            </p>
          </section>

          <section className="space-y-3">
            <h2 className="font-display text-lg tracking-[0.15em] text-accent uppercase">
              Workflow
            </h2>
            <ol className="flex flex-col gap-3">
              {steps.map((step, index) => (
                <li
                  key={step.title}
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
                </li>
              ))}
            </ol>
          </section>

          <section className="space-y-3 rounded-lg border border-accent/25 bg-accent/5 px-4 py-4">
            <h2 className="font-display text-lg tracking-[0.15em] text-accent uppercase">
              Install the plugin
            </h2>
            <p className="font-serif text-sm leading-relaxed text-foreground/75">
              Choose your Mod Organizer 2{" "}
              <span className="font-mono text-xs">plugins</span> folder — usually
              something like{" "}
              <code className="rounded bg-background/60 px-1 py-0.5 font-mono text-[0.7rem]">
                Mod Organizer 2\plugins
              </code>
              . Nerevar will write{" "}
              <code className="rounded bg-background/60 px-1 py-0.5 font-mono text-[0.7rem]">
                export_modlist_with_directories.py
              </code>{" "}
              there. Restart MO2 if it was already running.
            </p>
            <Button
              variant="launch"
              className={cn(
                "h-10 w-full font-display text-base tracking-[0.15em] uppercase",
              )}
              disabled={installing}
              onClick={() => void installPlugin()}
            >
              {installing ? (
                <Loader2 className="animate-spin" data-icon="inline-start" />
              ) : (
                <Download data-icon="inline-start" />
              )}
              Choose plugins folder and install
            </Button>
          </section>

          <section className="space-y-2">
            <h2 className="font-display text-lg tracking-[0.15em] text-accent uppercase">
              After installing
            </h2>
            <ul className="flex flex-col gap-2 font-serif text-sm leading-relaxed text-foreground/70">
              <li className="flex items-start gap-2">
                <ListOrdered className="mt-0.5 size-4 shrink-0 text-accent/70" />
                In MO2, open the Tools menu and run{" "}
                <span className="font-mono text-xs">
                  Export Enabled Mods CSV for Nerevar
                </span>
                .
              </li>
              <li className="flex items-start gap-2">
                <FolderOpen className="mt-0.5 size-4 shrink-0 text-accent/70" />
                Save the CSV anywhere convenient, then import it from your
                instance data manager in Nerevar.
              </li>
            </ul>
          </section>
        </CardContent>
      </Card>
    </div>
  );
}
