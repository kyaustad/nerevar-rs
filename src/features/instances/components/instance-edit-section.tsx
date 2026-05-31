import { Button } from "@/components/ui/button";
import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import {
  instanceEditSchema,
  type InstanceEditFormValues,
} from "@/features/instances/schemas/instance-edit-schema";
import type { InstanceConnectionSettings, InstanceEditPayload } from "@/types";
import { standardSchemaResolver } from "@hookform/resolvers/standard-schema";
import { invoke } from "@tauri-apps/api/core";
import { Loader2, Save } from "lucide-react";
import { useEffect, useState } from "react";
import { useForm } from "react-hook-form";
import { toast } from "sonner";

const SECTION_LABEL =
  "text-sm font-display tracking-[0.15em] text-foreground/70 uppercase";

export function InstanceEditSection({
  instanceId,
  isSynced,
}: {
  instanceId: string;
  isSynced: boolean;
}) {
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);

  const form = useForm<InstanceEditFormValues>({
    resolver: standardSchemaResolver(instanceEditSchema),
    defaultValues: {
      name: "",
      description: "",
      host: "",
      port: 25567,
      password: "",
    },
  });

  useEffect(() => {
    let cancelled = false;

    const load = async () => {
      setLoading(true);
      try {
        const settings = await invoke<InstanceConnectionSettings>(
          "get_instance_connection_settings",
          { instanceId },
        );
        if (cancelled) return;
        form.reset({
          name: settings.name,
          description: settings.description,
          host: settings.host,
          port: settings.port,
          password: settings.password,
        });
      } catch (error) {
        if (!cancelled) {
          toast.error(`Failed to load instance settings: ${error}`);
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    };

    void load();
    return () => {
      cancelled = true;
    };
  }, [form, instanceId]);

  const onSubmit = form.handleSubmit(async (values) => {
    setSaving(true);
    try {
      const payload: InstanceEditPayload = {
        id: instanceId,
        name: values.name,
        description: values.description,
        host: values.host,
        port: values.port,
        password: values.password,
      };
      await invoke<InstanceConnectionSettings>("update_instance", { edit: payload });
      toast.success("Instance settings saved");
    } catch (error) {
      toast.error(String(error));
    } finally {
      setSaving(false);
    }
  });

  const hostLabel = isSynced ? "Nerevar host" : "Server hostname";
  const portLabel = isSynced ? "Nerevar sync port" : "Game port";

  return (
    <form onSubmit={onSubmit} className="flex flex-col gap-4">
      <Field>
        <FieldLabel className={SECTION_LABEL} htmlFor={`${instanceId}-name`}>
          Name
        </FieldLabel>
        <Input
          id={`${instanceId}-name`}
          disabled={loading || saving}
          {...form.register("name")}
        />
        {form.formState.errors.name ? (
          <FieldError errors={[form.formState.errors.name]} />
        ) : null}
      </Field>

      <Field>
        <FieldLabel
          className={SECTION_LABEL}
          htmlFor={`${instanceId}-description`}
        >
          Description
        </FieldLabel>
        <Textarea
          id={`${instanceId}-description`}
          disabled={loading || saving}
          {...form.register("description")}
        />
        {form.formState.errors.description ? (
          <FieldError errors={[form.formState.errors.description]} />
        ) : null}
      </Field>

      <div className="grid gap-4 sm:grid-cols-2">
        <Field>
          <FieldLabel className={SECTION_LABEL} htmlFor={`${instanceId}-host`}>
            {hostLabel}
          </FieldLabel>
          <Input
            id={`${instanceId}-host`}
            disabled={loading || saving}
            {...form.register("host")}
          />
          {form.formState.errors.host ? (
            <FieldError errors={[form.formState.errors.host]} />
          ) : null}
        </Field>

        <Field>
          <FieldLabel className={SECTION_LABEL} htmlFor={`${instanceId}-port`}>
            {portLabel}
          </FieldLabel>
          <Input
            id={`${instanceId}-port`}
            type="number"
            disabled={loading || saving}
            {...form.register("port", { valueAsNumber: true })}
          />
          {form.formState.errors.port ? (
            <FieldError errors={[form.formState.errors.port]} />
          ) : null}
        </Field>
      </div>

      <Field>
        <FieldLabel
          className={SECTION_LABEL}
          htmlFor={`${instanceId}-password`}
        >
          Password
        </FieldLabel>
        <Input
          id={`${instanceId}-password`}
          type="password"
          autoComplete="off"
          disabled={loading || saving}
          {...form.register("password")}
        />
        <p className="font-serif text-sm text-foreground/60">
          {isSynced
            ? "Used for Nerevar sync and TES3MP client connection. Leave blank if the host has no password."
            : "Written to tes3mp-server-default.cfg. Leave blank for no password."}
        </p>
        {form.formState.errors.password ? (
          <FieldError errors={[form.formState.errors.password]} />
        ) : null}
      </Field>

      <Button
        type="submit"
        variant="outline"
        className="h-10 w-full font-display tracking-[0.15em] uppercase"
        disabled={loading || saving}
      >
        {saving ? (
          <Loader2 className="animate-spin" data-icon="inline-start" />
        ) : (
          <Save data-icon="inline-start" />
        )}
        Save settings
      </Button>
    </form>
  );
}
