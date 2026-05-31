import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@/components/ui/alert-dialog";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
import type { InstanceConfig } from "@/types";
import { invoke } from "@tauri-apps/api/core";
import { Loader2, Trash2 } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";
import { useLocation } from "wouter";

export function DeleteInstanceSection({
  instance,
  backHref,
}: {
  instance: InstanceConfig;
  backHref: string;
}) {
  const [, navigate] = useLocation();
  const [open, setOpen] = useState(false);
  const [deleteDataDirectory, setDeleteDataDirectory] = useState(false);
  const [deleting, setDeleting] = useState(false);

  const confirmDelete = async () => {
    setDeleting(true);
    try {
      await invoke("delete_instance", {
        instanceId: instance.id,
        deleteDataDirectory,
      });
      toast.success(`Deleted "${instance.name}"`);
      setOpen(false);
      navigate(backHref);
    } catch (error) {
      toast.error(String(error));
    } finally {
      setDeleting(false);
    }
  };

  return (
    <AlertDialog
      open={open}
      onOpenChange={(nextOpen) => {
        if (!deleting) {
          setOpen(nextOpen);
          if (!nextOpen) {
            setDeleteDataDirectory(false);
          }
        }
      }}
    >
      <AlertDialogTrigger asChild>
        <Button
          variant="outline"
          className="h-10 w-full border-destructive/40 font-display tracking-[0.15em] text-destructive uppercase hover:border-destructive/60 hover:bg-destructive/5 hover:text-destructive"
        >
          <Trash2 data-icon="inline-start" />
          Delete instance
        </Button>
      </AlertDialogTrigger>
      <AlertDialogContent className="data-[size=default]:max-w-md data-[size=default]:sm:max-w-lg">
        <AlertDialogHeader className="text-left">
          <AlertDialogTitle className="font-display text-xl tracking-[0.06em]">
            Delete instance?
          </AlertDialogTitle>
          <AlertDialogDescription asChild>
            <div className="space-y-3 text-left text-sm leading-relaxed text-muted-foreground">
              <p>
                This removes <strong>{instance.name}</strong> from Nerevar. The
                instance will no longer appear in your lists or settings.
              </p>
              <div className="rounded-md border border-destructive/30 bg-destructive/5 px-3 py-2">
                <p className="font-mono text-xs text-foreground/70 break-all">
                  {instance.path}
                </p>
              </div>
              <div className="flex items-start gap-3 rounded-md border border-border/60 bg-background/30 px-3 py-3">
                <Checkbox
                  id={`delete-data-${instance.id}`}
                  checked={deleteDataDirectory}
                  disabled={deleting}
                  onCheckedChange={(checked) =>
                    setDeleteDataDirectory(checked === true)
                  }
                />
                <div className="space-y-1">
                  <Label
                    htmlFor={`delete-data-${instance.id}`}
                    className="font-display text-sm tracking-[0.08em] text-foreground"
                  >
                    Also delete instance folder from disk
                  </Label>
                  <p className="font-serif text-sm text-foreground/65">
                    Permanently removes the instance directory, including TES3MP
                    files and mod data. This cannot be undone.
                  </p>
                </div>
              </div>
              {!deleteDataDirectory ? (
                <p>
                  With the box unchecked, only the Nerevar entry is removed.
                  Files on disk are left in place.
                </p>
              ) : null}
            </div>
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel disabled={deleting}>Cancel</AlertDialogCancel>
          <AlertDialogAction
            variant="destructive"
            className="text-primary-foreground"
            disabled={deleting}
            onClick={(event) => {
              event.preventDefault();
              void confirmDelete();
            }}
          >
            {deleting ? (
              <Loader2 className="animate-spin" data-icon="inline-start" />
            ) : (
              <Trash2 data-icon="inline-start" />
            )}
            Delete instance
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
