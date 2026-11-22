import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import type { NexusAccountStatus, NexusLinkHandlerStatus } from "@/types";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { ExternalLink, KeyRound, Loader2, Unlink } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { toast } from "sonner";

const API_KEYS_PAGE = "https://next.nexusmods.com/settings/api-keys";

/** Nexus Mods account (personal API key) and the nxm:// link handler. */
export function NexusAccountSection() {
  const [status, setStatus] = useState<NexusAccountStatus | null>(null);
  const [handler, setHandler] = useState<NexusLinkHandlerStatus | null>(null);
  const [apiKey, setApiKey] = useState("");
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    try {
      const [account, links] = await Promise.all([
        invoke<NexusAccountStatus>("nexus_account_status"),
        invoke<NexusLinkHandlerStatus>("nexus_link_handler_status"),
      ]);
      setStatus(account);
      setHandler(links);
    } catch (error) {
      toast.error(`Failed to read the Nexus Mods account: ${error}`);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const link = async () => {
    setBusy(true);
    try {
      setStatus(
        await invoke<NexusAccountStatus>("nexus_link_api_key", { apiKey }),
      );
      setApiKey("");
      toast.success("Nexus Mods account linked");
    } catch (error) {
      toast.error(String(error));
    } finally {
      setBusy(false);
    }
  };

  const unlink = async () => {
    setBusy(true);
    try {
      setStatus(await invoke<NexusAccountStatus>("nexus_unlink"));
      toast.success("Nexus Mods account unlinked");
    } catch (error) {
      toast.error(String(error));
    } finally {
      setBusy(false);
    }
  };

  const toggleHandler = async (enabled: boolean) => {
    try {
      setHandler(
        await invoke<NexusLinkHandlerStatus>("nexus_set_link_handler", {
          enabled,
        }),
      );
      toast.success(
        enabled
          ? "Nexus “Mod Manager Download” buttons now open Nerevar"
          : "Nerevar no longer handles Nexus downloads",
      );
    } catch (error) {
      toast.error(String(error));
    }
  };

  return (
    <section className="space-y-4">
      <div className="flex flex-wrap items-center gap-2">
        <h2 className="font-display text-lg tracking-[0.15em] text-accent uppercase">
          Nexus Mods
        </h2>
        {status?.linked && status.name ? (
          <Badge variant="default" className="font-mono text-[0.65rem]">
            {status.name} · {status.premium ? "Premium" : "Free"}
          </Badge>
        ) : (
          <Badge variant="secondary" className="font-mono text-[0.65rem]">
            Not linked
          </Badge>
        )}
      </div>
      <p className="font-serif text-sm leading-relaxed text-foreground/70">
        Nerevar downloads the Nexus mods of Modding-OpenMW lists, and the mods
        you download with Nexus&apos; &ldquo;Mod Manager Download&rdquo; button,
        with your personal API key. It is stored in your system&apos;s
        credential manager. Premium accounts download automatically; free
        accounts click &ldquo;Mod Manager Download&rdquo; on each mod&apos;s
        page, which Nerevar opens for you.
      </p>
      {status?.error ? (
        <p className="font-serif text-sm text-destructive">{status.error}</p>
      ) : null}

      {status?.linked ? (
        <Button
          type="button"
          variant="outline"
          disabled={busy}
          onClick={() => void unlink()}
        >
          <Unlink data-icon="inline-start" />
          Unlink account
        </Button>
      ) : (
        <div className="space-y-2">
          <Label
            htmlFor="nexus-api-key"
            className="font-display text-[0.75rem] tracking-[0.2em] uppercase text-foreground/70"
          >
            Personal API key
          </Label>
          <div className="flex flex-wrap gap-2">
            <Input
              id="nexus-api-key"
              type="password"
              autoComplete="off"
              value={apiKey}
              placeholder="Paste your key"
              className="min-w-64 flex-1 font-mono"
              onChange={(event) => setApiKey(event.target.value)}
            />
            <Button
              type="button"
              disabled={busy || apiKey.trim().length === 0}
              onClick={() => void link()}
            >
              {busy ? (
                <Loader2 className="animate-spin" data-icon="inline-start" />
              ) : (
                <KeyRound data-icon="inline-start" />
              )}
              Link
            </Button>
          </div>
          <Button
            type="button"
            variant="link"
            className="h-auto px-0 text-sm"
            onClick={() => void openUrl(API_KEYS_PAGE)}
          >
            Get your key at the bottom of Nexus&apos; API keys page
            <ExternalLink data-icon="inline-end" />
          </Button>
        </div>
      )}

      {handler?.supported ? (
        <div className="flex items-start justify-between gap-4 rounded-lg border border-border/50 bg-background/20 px-4 py-3">
          <div className="space-y-1">
            <Label
              htmlFor="nexus-link-handler"
              className="font-display text-sm tracking-[0.08em] text-foreground"
            >
              Nerevar handles Nexus downloads
            </Label>
            <p className="font-serif text-sm text-foreground/65">
              &ldquo;Mod Manager Download&rdquo; buttons (nxm:// links) open
              Nerevar, which installs the mod into the instance whose data
              manager is open. This replaces Mod Organizer 2, Vortex or umo as
              the handler until one of them takes it back.
            </p>
          </div>
          <Switch
            id="nexus-link-handler"
            checked={handler.registered}
            onCheckedChange={(checked) => void toggleHandler(checked)}
          />
        </div>
      ) : null}
    </section>
  );
}
