import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectItemText,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type {
  MomwInstallReport,
  MomwManualMod,
  MomwModlistSummary,
  NexusAccountStatus,
} from "@/types";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  CheckCircle2,
  ExternalLink,
  FilePlus2,
  Loader2,
  TriangleAlert,
} from "lucide-react";
import { useEffect, useState } from "react";
import { toast } from "sonner";
import { Link } from "wouter";

function listTitle(slug: string) {
  return slug
    .split("-")
    .map((word) =>
      word === "wip" ? "(WIP)" : word[0].toUpperCase() + word.slice(1),
    )
    .join(" ");
}

/**
 * Installs a Modding-OpenMW.com list into the instance. The list's mods become ordinary packages that can be
 * reordered, disabled or removed afterwards, like a Mod Organizer 2 profile made from the list.
 */
export function MomwInstallDialog({
  open,
  installing,
  onOpenChange,
  onInstall,
  report,
}: {
  open: boolean;
  installing: boolean;
  onOpenChange: (open: boolean) => void;
  onInstall: (slug: string, beta: boolean) => void;
  report: MomwInstallReport | null;
}) {
  const [beta, setBeta] = useState(false);
  const [lists, setLists] = useState<MomwModlistSummary[]>([]);
  const [slug, setSlug] = useState("");
  const [account, setAccount] = useState<NexusAccountStatus | null>(null);

  useEffect(() => {
    if (!open) return;
    invoke<NexusAccountStatus>("nexus_account_status")
      .then(setAccount)
      .catch(() => setAccount(null));
    invoke<MomwModlistSummary[]>("momw_list_modlists", { beta })
      .then((result) => {
        setLists(result);
        setSlug((current) =>
          result.some((list) => list.slug === current) ? current : "",
        );
      })
      .catch((error) =>
        toast.error(`Couldn't reach Modding-OpenMW.com: ${error}`),
      );
  }, [open, beta]);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle className="font-display tracking-[0.08em]">
            Install a Modding-OpenMW list
          </DialogTitle>
          <DialogDescription className="font-serif text-left leading-relaxed">
            Downloads a list&apos;s mods from{" "}
            <button
              type="button"
              className="underline"
              onClick={() => void openUrl("https://modding-openmw.com")}
            >
              Modding-OpenMW.com
            </button>{" "}
            into this instance and sets its load order and settings, like umo
            and momw-configurator. Afterwards the mods are normal packages you
            can reorder, disable, remove or add to. Running it again resumes it
            and updates mods whose files changed.
          </DialogDescription>
        </DialogHeader>

        {report ? (
          <MomwReport report={report} />
        ) : (
          <div className="space-y-4">
            {!account?.linked ? (
              <p className="rounded-md border border-amber-500/40 bg-amber-500/10 px-3 py-2 font-serif text-sm text-amber-200">
                Link your Nexus Mods account in{" "}
                <Link href="/settings" className="underline">
                  Nerevar&apos;s settings
                </Link>{" "}
                first: almost all of the lists&apos; mods are on Nexus.
              </p>
            ) : !account.premium ? (
              <p className="rounded-md border border-border/50 bg-background/30 px-3 py-2 font-serif text-sm text-foreground/75">
                With a free Nexus account, Nerevar opens each Nexus mod&apos;s
                page in turn: click &ldquo;Mod Manager Download&rdquo; there and
                the install continues. Turn on &ldquo;Nerevar handles Nexus
                downloads&rdquo; in settings for this.
              </p>
            ) : null}
            <div className="space-y-2">
              <Label className="font-display text-[0.75rem] tracking-[0.2em] uppercase text-foreground/70">
                List
              </Label>
              <Select value={slug} onValueChange={setSlug}>
                <SelectTrigger className="w-full">
                  <SelectValue placeholder="Choose a list" />
                </SelectTrigger>
                <SelectContent position="popper" className="max-h-72">
                  {lists.map((list) => (
                    <SelectItem key={list.slug} value={list.slug}>
                      <SelectItemText>{listTitle(list.slug)}</SelectItemText>
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <label className="flex items-center gap-2 font-serif text-sm text-foreground/75">
              <Checkbox
                checked={beta}
                onCheckedChange={(checked) => setBeta(checked === true)}
              />
              Use the lists of beta.modding-openmw.com
            </label>
          </div>
        )}

        <DialogFooter>
          {report ? (
            reportNeedsAttention(report) ? (
              <>
                <Button
                  type="button"
                  variant="outline"
                  onClick={() => onOpenChange(false)}
                >
                  Close
                </Button>
                <Button
                  type="button"
                  disabled={installing}
                  onClick={() => onInstall(report.modlist, report.beta)}
                >
                  {installing ? (
                    <Loader2
                      className="animate-spin"
                      data-icon="inline-start"
                    />
                  ) : null}
                  Install again
                </Button>
              </>
            ) : (
              <Button type="button" onClick={() => onOpenChange(false)}>
                Done
              </Button>
            )
          ) : (
            <Button
              type="button"
              disabled={!slug || installing}
              onClick={() => onInstall(slug, beta)}
            >
              {installing ? (
                <Loader2 className="animate-spin" data-icon="inline-start" />
              ) : null}
              Install {slug ? listTitle(slug) : "list"}
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

/** Whether the player has something to do after an install. */
export function reportNeedsAttention(report: MomwInstallReport) {
  return (
    report.manual.length > 0 ||
    report.failed.length > 0 ||
    report.leftOut.length > 0
  );
}

function MomwReport({ report }: { report: MomwInstallReport }) {
  const [added, setAdded] = useState<Set<string>>(new Set());
  const [adding, setAdding] = useState<string | null>(null);

  const addArchive = async (mod: MomwManualMod) => {
    const [archive] = await invoke<string[]>("open_archive_file_picker");
    if (!archive) return;
    setAdding(mod.key);
    try {
      await invoke("momw_add_manual_archive", { key: mod.key, archive });
      setAdded((current) => new Set(current).add(mod.key));
      toast.success(`${mod.name} added; install the list again to finish`);
    } catch (error) {
      toast.error(String(error));
    } finally {
      setAdding(null);
    }
  };

  return (
    <div className="max-h-[60vh] space-y-5 overflow-y-auto pr-1 font-serif text-sm">
      <p className="text-foreground/80">
        {report.installed} mods installed, {report.upToDate} already up to date,{" "}
        {report.packages} packages in the load order.
        {report.repaired.length > 0
          ? ` Repaired ${report.repaired.length} mods whose files had changed or gone missing.`
          : ""}
      </p>

      {reportNeedsAttention(report) ? (
        <p className="flex items-start gap-2 rounded-md border border-amber-500/40 bg-amber-500/10 px-3 py-2 text-amber-200">
          <TriangleAlert className="mt-0.5 size-4 shrink-0" />
          <span>
            The list isn&apos;t complete yet. The game runs without the parts
            below; follow the steps, then press &ldquo;Install again&rdquo; to
            add them. Nothing already downloaded is downloaded again.
          </span>
        </p>
      ) : (
        <p className="flex items-center gap-2 text-emerald-300">
          <CheckCircle2 className="size-4" />
          Everything in the list is installed.
        </p>
      )}

      {report.manual.length > 0 ? (
        <section className="space-y-2">
          <h3 className="font-display text-sm tracking-[0.12em] text-accent uppercase">
            Download these yourself ({report.manual.length})
          </h3>
          <ol className="list-decimal space-y-0.5 pl-5 text-foreground/65">
            <li>Open the mod&apos;s page and download its file.</li>
            <li>
              Press &ldquo;Add downloaded file&rdquo; and choose the archive.
              Nerevar keeps it in your downloads folder for every instance.
            </li>
            <li>Press &ldquo;Install again&rdquo; below.</li>
          </ol>
          <ul className="space-y-1.5">
            {report.manual.map((mod) => (
              <li
                key={mod.key}
                className="space-y-1.5 rounded-md border border-border/40 bg-card/40 px-3 py-2"
              >
                <p className="font-display text-foreground">{mod.name}</p>
                <p className="text-foreground/60">{mod.reason}</p>
                <div className="flex flex-wrap gap-2">
                  <Button
                    type="button"
                    size="sm"
                    variant="outline"
                    onClick={() => void openUrl(mod.url)}
                  >
                    <ExternalLink data-icon="inline-start" />
                    Open page
                  </Button>
                  {added.has(mod.key) ? (
                    <span className="inline-flex items-center gap-1 text-emerald-300">
                      <CheckCircle2 className="size-4" />
                      Added
                    </span>
                  ) : (
                    <Button
                      type="button"
                      size="sm"
                      disabled={adding !== null}
                      onClick={() => void addArchive(mod)}
                    >
                      {adding === mod.key ? (
                        <Loader2
                          className="animate-spin"
                          data-icon="inline-start"
                        />
                      ) : (
                        <FilePlus2 data-icon="inline-start" />
                      )}
                      Add downloaded file
                    </Button>
                  )}
                </div>
              </li>
            ))}
          </ul>
        </section>
      ) : null}

      {report.leftOut.length > 0 ? (
        <section className="space-y-2">
          <h3 className="font-display text-sm tracking-[0.12em] text-accent uppercase">
            Plugins turned off ({report.leftOut.length})
          </h3>
          <p className="text-foreground/65">
            These need a plugin that isn&apos;t installed, so they&apos;re off
            to keep the game from stopping with &ldquo;asks for parent
            file&rdquo;. They turn back on when you install the list again after
            adding the mod with that plugin.
          </p>
          <ul className="space-y-1">
            {report.leftOut.map((item) => (
              <li key={item.plugin}>
                <code className="text-foreground">{item.plugin}</code>{" "}
                <span className="text-foreground/60">needs</span>{" "}
                <code className="text-foreground">{item.master}</code>
              </li>
            ))}
          </ul>
        </section>
      ) : null}

      {report.failed.length > 0 ? (
        <section className="space-y-2">
          <h3 className="font-display text-sm tracking-[0.12em] text-destructive uppercase">
            Failed ({report.failed.length})
          </h3>
          <p className="text-foreground/65">
            Usually a download that broke off or a site that was down: install
            the list again to retry them.
          </p>
          <ul className="space-y-1">
            {report.failed.map((mod) => (
              <li key={mod.name}>
                <span className="text-foreground">{mod.name}</span>:{" "}
                <span className="text-foreground/60">{mod.error}</span>
              </li>
            ))}
          </ul>
        </section>
      ) : null}

      {report.notes.length > 0 ? (
        <section className="space-y-1">
          <h3 className="font-display text-sm tracking-[0.12em] text-foreground/70 uppercase">
            Notes
          </h3>
          <ul className="list-disc space-y-0.5 pl-5 text-foreground/60">
            {report.notes.map((note) => (
              <li key={note}>{note}</li>
            ))}
          </ul>
        </section>
      ) : null}
    </div>
  );
}
