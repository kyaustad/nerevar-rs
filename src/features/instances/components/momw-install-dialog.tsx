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
  MomwModlistSummary,
  NexusAccountStatus,
} from "@/types";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { ExternalLink, Loader2 } from "lucide-react";
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
            <Button type="button" onClick={() => onOpenChange(false)}>
              Done
            </Button>
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

function MomwReport({ report }: { report: MomwInstallReport }) {
  return (
    <div className="max-h-[55vh] space-y-4 overflow-y-auto pr-1 font-serif text-sm">
      <p className="text-foreground/80">
        {report.installed} mods installed, {report.upToDate} already up to date,{" "}
        {report.packages} packages in the load order.
      </p>
      {report.manual.length > 0 ? (
        <section className="space-y-2">
          <h3 className="font-display text-sm tracking-[0.12em] text-accent uppercase">
            Download these yourself ({report.manual.length})
          </h3>
          <p className="text-foreground/65">
            Extract each into the folder shown (inside the instance&apos;s data
            folder), or install its archive with &ldquo;Install archive&rdquo;
            and rename the package, then install the list again to finish the
            load order.
          </p>
          <ul className="space-y-1.5">
            {report.manual.map((mod) => (
              <li
                key={mod.name}
                className="rounded-md border border-border/40 bg-card/40 px-3 py-2"
              >
                <button
                  type="button"
                  className="inline-flex items-center gap-1 font-display text-foreground underline-offset-2 hover:underline"
                  onClick={() => void openUrl(mod.url)}
                >
                  {mod.name}
                  <ExternalLink className="size-3.5" />
                </button>
                <p className="text-foreground/60">{mod.reason}</p>
                <code className="text-xs text-foreground/50">{mod.folder}</code>
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
