import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { invoke } from "@tauri-apps/api/core";
import { FolderOpen, RotateCcw } from "lucide-react";
import { useEffect, useState } from "react";
import { toast } from "sonner";

/** Folder of the archives modlist installs download, shared by every instance. */
export function DownloadsFolderSection() {
  const [folder, setFolder] = useState("");

  useEffect(() => {
    invoke<string>("momw_get_downloads_dir")
      .then(setFolder)
      .catch((error) => toast.error(String(error)));
  }, []);

  const choose = async (path: string | null) => {
    try {
      setFolder(await invoke<string>("momw_set_downloads_dir", { path }));
      toast.success("Downloads folder saved");
    } catch (error) {
      toast.error(String(error));
    }
  };

  const pick = async () => {
    try {
      await choose(await invoke<string>("open_directory_picker"));
    } catch {
      // No folder chosen
    }
  };

  return (
    <section className="space-y-4">
      <h2 className="font-display text-lg tracking-[0.15em] text-accent uppercase">
        Downloads
      </h2>
      <p className="font-serif text-sm leading-relaxed text-foreground/70">
        Modlist installs keep every archive they download here, for all your
        instances. Installing a list again repairs mods whose files changed or
        went missing and updates mods with new versions, downloading only what
        isn&apos;t here yet. Installing it into another instance reuses the same
        archives.
      </p>
      <div className="flex flex-wrap gap-2">
        <Input
          readOnly
          value={folder}
          className="min-w-64 flex-1 font-mono text-xs"
        />
        <Button type="button" variant="outline" onClick={() => void pick()}>
          <FolderOpen data-icon="inline-start" />
          Choose
        </Button>
        <Button type="button" variant="ghost" onClick={() => void choose(null)}>
          <RotateCcw data-icon="inline-start" />
          Default
        </Button>
      </div>
    </section>
  );
}
