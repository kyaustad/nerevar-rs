import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Separator } from "@/components/ui/separator";
import { Textarea } from "@/components/ui/textarea";
import { useConfig } from "@/features/config/context/config-context-provider";
import {
  newInstanceDefaultValues,
  newInstanceSchema,
  type NewInstanceFormValues,
} from "@/features/instances/schemas/new-instance-schema";
import { ReleaseSelector } from "@/features/tes3mp-releases/components/release-selector";
import { cn } from "@/lib/utils";
import { standardSchemaResolver } from "@hookform/resolvers/standard-schema";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { ArrowLeft, Loader2, Sparkles } from "lucide-react";
import { useEffect } from "react";
import { Controller, useForm, type FieldErrors } from "react-hook-form";
import { toast } from "sonner";
import { navigate } from "wouter/use-browser-location";
import type { NewInstanceConfig } from "@/types/NewInstanceConfig";

const SECTION_LABEL =
  "text-lg font-light font-display tracking-[0.08em] text-foreground";
const SECTION_DESC =
  "text-xs text-foreground/75 font-light tracking-[0.1em] font-sans text-left leading-loose";
const NESTED_LABEL =
  "text-base font-light font-display tracking-[0.08em] text-foreground";

function InstanceFormField({
  id,
  label,
  description,
  nested,
  invalid,
  error,
  children,
}: {
  id: string;
  label: string;
  description?: React.ReactNode;
  nested?: boolean;
  invalid?: boolean;
  error?: { message?: string };
  children: React.ReactNode;
}) {
  return (
    <Field
      data-invalid={invalid}
      className={cn("flex flex-col gap-2", nested && "pl-8 mt-4")}
    >
      <FieldLabel
        htmlFor={id}
        className={nested ? NESTED_LABEL : SECTION_LABEL}
      >
        {label}
      </FieldLabel>
      {description &&
        (typeof description === "string" ? (
          <p className={SECTION_DESC}>{description}</p>
        ) : (
          description
        ))}
      {children}
      {invalid && error && <FieldError errors={[error]} />}
    </Field>
  );
}

function firstValidationMessage(
  errors: FieldErrors<NewInstanceFormValues>,
): string | undefined {
  for (const error of Object.values(errors)) {
    if (error && typeof error === "object" && "message" in error) {
      const message = error.message;
      if (typeof message === "string") {
        return message;
      }
    }
  }
  return undefined;
}

function buildInstanceRootPath(rootPath: string, instanceName: string): string {
  const base = rootPath.replace(/\//g, "\\").replace(/\\+$/, "");
  const folder = instanceName
    .trim()
    .replace(/[<>:"/\\|?*]/g, "")
    .trim();
  if (!base) return folder;
  if (!folder) return base;
  return `${base}\\${folder}`;
}

function buildInstanceDataDir(rootPath: string, instanceName: string): string {
  return `${rootPath}\\${instanceName}\\data`;
}

export function NewInstancePage() {
  const config = useConfig();

  const form = useForm<NewInstanceFormValues>({
    resolver: standardSchemaResolver(newInstanceSchema),
    defaultValues: {
      ...newInstanceDefaultValues,
      instanceRootPath: buildInstanceRootPath(config?.rootPath ?? "", ""),
      instanceDataDir: buildInstanceDataDir(config?.rootPath ?? "", ""),
    },
    mode: "onSubmit",
    reValidateMode: "onChange",
  });

  useEffect(() => {
    const unlisten = listen("on_config_added_instance", () => {
      toast.success("Instance created successfully");
      navigate("/owned-instances");
    });
    return () => {
      unlisten.then((unlistenFn) => unlistenFn());
    };
  }, []);

  const instanceName = form.watch("instanceName");
  const nerevarRoot = config?.rootPath ?? "";

  useEffect(() => {
    form.setValue(
      "instanceRootPath",
      buildInstanceRootPath(nerevarRoot, instanceName),
      { shouldValidate: true },
    );
    form.setValue(
      "instanceDataDir",
      buildInstanceDataDir(nerevarRoot, instanceName),
      { shouldValidate: true },
    );
  }, [nerevarRoot, instanceName, form]);

  const onSubmit = async (data: NewInstanceFormValues) => {
    try {
      await invoke<void>("add_instance", {
        newInstance: {
          releaseId: data.releaseId,
          instanceName: data.instanceName,
          instanceDescription: data.instanceDescription,
          instanceRootPath: data.instanceRootPath,
          instanceDataDir: data.instanceDataDir,
          serverHostName: data.serverHostName,
          maxPlayers: data.maxPlayers,
          serverPort: data.serverPort,
          password: data.password,
          masterServerEnabled: data.masterServerEnabled,
        } as NewInstanceConfig,
      });
    } catch (error) {
      toast.error(
        typeof error === "string" ? error : "Failed to create instance",
      );
    }
  };

  const onInvalid = (errors: FieldErrors<NewInstanceFormValues>) => {
    const message =
      firstValidationMessage(errors) ??
      "Please fix the highlighted fields before creating the instance.";
    toast.error(message);
  };

  const isSubmitting = form.formState.isSubmitting;

  return (
    <div className="mx-auto flex w-full max-w-6xl flex-col items-center gap-6 py-8 text-center">
      <Button
        variant="server"
        size="lg"
        className="w-full max-w-sm"
        onClick={() => navigate("/")}
      >
        <ArrowLeft data-icon="inline-start" />
        Go back
      </Button>
      <h1 className="font-display text-2xl tracking-[0.08em] text-gradient-gold">
        Create a new instance
      </h1>

      <Card className="w-full max-w-2xl" disableHover disableTap>
        <CardContent>
          <form
            id="new-instance-form"
            onSubmit={form.handleSubmit(onSubmit, onInvalid)}
            className="flex flex-col"
          >
            <Controller
              name="releaseId"
              control={form.control}
              render={({ field, fieldState }) => (
                <InstanceFormField
                  id="new-instance-release"
                  label="TES3MP Release"
                  description="This is the version of TES3MP that will be used for this instance. The latest non-VR release TES3MP 0.8.1 is the only supported release currently. I don't really plan on supporting older releases."
                  invalid={fieldState.invalid}
                  error={fieldState.error}
                >
                  <ReleaseSelector
                    value={field.value}
                    onValueChange={field.onChange}
                  />
                </InstanceFormField>
              )}
            />
            <Separator className="my-4" />
            <Controller
              name="instanceName"
              control={form.control}
              render={({ field, fieldState }) => (
                <InstanceFormField
                  id="new-instance-name"
                  label="Instance Name"
                  description="This is the name of the instance. It will be used to identify the instance in the UI and in the file system."
                  invalid={fieldState.invalid}
                  error={fieldState.error}
                >
                  <Input
                    {...field}
                    id="new-instance-name"
                    aria-invalid={fieldState.invalid}
                  />
                </InstanceFormField>
              )}
            />
            <Separator className="my-4" />
            <Controller
              name="instanceDescription"
              control={form.control}
              render={({ field, fieldState }) => (
                <InstanceFormField
                  id="new-instance-description"
                  label="Instance Description"
                  description="This is the description of the instance. It will be used to provide more information about the instance."
                  invalid={fieldState.invalid}
                  error={fieldState.error}
                >
                  <Textarea
                    {...field}
                    id="new-instance-description"
                    aria-invalid={fieldState.invalid}
                  />
                </InstanceFormField>
              )}
            />
            <Separator className="my-4" />
            <Controller
              name="instanceRootPath"
              control={form.control}
              render={({ field, fieldState }) => (
                <InstanceFormField
                  id="new-instance-root-path"
                  label="Instance Root Path"
                  description="This is the root path of the instance. It will be used to store the instance data. This path is derived from your Nerevar data directory and the instance name above."
                  invalid={fieldState.invalid}
                  error={fieldState.error}
                >
                  <Input
                    {...field}
                    id="new-instance-root-path"
                    readOnly
                    disabled
                    aria-invalid={fieldState.invalid}
                  />
                </InstanceFormField>
              )}
            />
            <Separator className="my-4" />
            <Controller
              name="instanceDataDir"
              control={form.control}
              render={({ field, fieldState }) => (
                <InstanceFormField
                  id="new-instance-data-dir"
                  label="Instance Data Directory"
                  description="This is the data directory of the instance. It will be where data files such as mods and plugins will be stored and can be seperated from the instance root path to allow for easier storage management. If you used Wabbajack or MO2 to install your mods, you can point this to the 'mods' folder in your installation directory containing the directories for your mods."
                  invalid={fieldState.invalid}
                  error={fieldState.error}
                >
                  <div className="flex gap-2">
                    <Input
                      id="new-instance-data-dir"
                      readOnly
                      value={field.value}
                      onChange={field.onChange}
                      className="font-mono text-xs bg-input/40"
                    />
                    <Button
                      type="button"
                      variant="outline"
                      className="shrink-0 font-display text-[0.75rem] tracking-[0.3em] uppercase"
                      onClick={() =>
                        invoke<string>("open_directory_picker").then((path) =>
                          field.onChange(path),
                        )
                      }
                      disabled={isSubmitting}
                    >
                      Browse
                    </Button>
                  </div>
                </InstanceFormField>
              )}
            />
            <Separator className="my-4" />

            <div className="flex flex-col gap-2">
              <Label className={SECTION_LABEL}>Server Defaults</Label>
              <p className={SECTION_DESC}>
                {`This is the default server settings for the instance. These values will be used to populate the`}{" "}
                <code className="font-mono text-accent bg-secondary/70 p-1">
                  tes3mp-server-defaults.cfg
                </code>{" "}
                {` file after setting up your instance.`}
              </p>

              <Controller
                name="serverHostName"
                control={form.control}
                render={({ field, fieldState }) => (
                  <InstanceFormField
                    id="new-instance-server-host-name"
                    label="Server Host Name"
                    nested
                    description="This is the hostname of your multiplayer server, and how other people will see its name in the TES3MP Server Browser and master server."
                    invalid={fieldState.invalid}
                    error={fieldState.error}
                  >
                    <Input
                      {...field}
                      id="new-instance-server-host-name"
                      autoCorrect="off"
                      aria-invalid={fieldState.invalid}
                    />
                  </InstanceFormField>
                )}
              />
              <Controller
                name="maxPlayers"
                control={form.control}
                render={({ field, fieldState }) => (
                  <InstanceFormField
                    id="new-instance-max-players"
                    label="Max Players"
                    nested
                    description="This is the maximum number of players that can connect to your server at once."
                    invalid={fieldState.invalid}
                    error={fieldState.error}
                  >
                    <Input
                      id="new-instance-max-players"
                      name={field.name}
                      type="number"
                      value={field.value}
                      autoCorrect="off"
                      aria-invalid={fieldState.invalid}
                      onBlur={field.onBlur}
                      ref={field.ref}
                      onChange={(event) => {
                        const next = event.target.valueAsNumber;
                        field.onChange(
                          Number.isFinite(next)
                            ? next
                            : newInstanceDefaultValues.maxPlayers,
                        );
                      }}
                    />
                  </InstanceFormField>
                )}
              />
              <Controller
                name="password"
                control={form.control}
                render={({ field, fieldState }) => (
                  <InstanceFormField
                    id="new-instance-password"
                    label="Server Password"
                    nested
                    description="This is the password that will be required to connect to your server. If left blank, no password will be required."
                    invalid={fieldState.invalid}
                    error={fieldState.error}
                  >
                    <Input
                      {...field}
                      id="new-instance-password"
                      type="password"
                      autoCorrect="off"
                      aria-invalid={fieldState.invalid}
                    />
                  </InstanceFormField>
                )}
              />
              <Controller
                name="serverPort"
                control={form.control}
                render={({ field, fieldState }) => (
                  <InstanceFormField
                    id="new-instance-server-port"
                    label="Server Port"
                    nested
                    description="This is the port that your server will listen on. If left blank, the default port of 25565 will be used."
                    invalid={fieldState.invalid}
                    error={fieldState.error}
                  >
                    <Input
                      id="new-instance-server-port"
                      name={field.name}
                      type="number"
                      value={field.value}
                      autoCorrect="off"
                      aria-invalid={fieldState.invalid}
                      onBlur={field.onBlur}
                      ref={field.ref}
                      onChange={(event) => {
                        const next = event.target.valueAsNumber;
                        field.onChange(
                          Number.isFinite(next)
                            ? next
                            : newInstanceDefaultValues.serverPort,
                        );
                      }}
                    />
                  </InstanceFormField>
                )}
              />
              <Controller
                name="masterServerEnabled"
                control={form.control}
                render={({ field, fieldState }) => (
                  <InstanceFormField
                    id="new-instance-master-server"
                    label="Show Server in Server Browser?"
                    nested
                    description={
                      <>
                        <p className={SECTION_DESC}>
                          This will report your server to the TES3MP Master
                          Server and allow it to show up in the TES3MP Server
                          Browser.
                        </p>
                        <p className="text-sm font-bold text-accent tracking-[0.05em] font-sans text-left leading-tight">
                          {`NOTE: If a user connects to your server not through Nerevar, you and them both lose the ability to sync data (mods) and use Nerevar's full feature set.`}
                        </p>
                      </>
                    }
                    invalid={fieldState.invalid}
                    error={fieldState.error}
                  >
                    <div className="flex flex-row items-center gap-2">
                      <Checkbox
                        id="new-instance-master-server"
                        name={field.name}
                        checked={field.value}
                        aria-invalid={fieldState.invalid}
                        onCheckedChange={(checked) =>
                          field.onChange(
                            checked === "indeterminate" ? false : checked,
                          )
                        }
                      />
                      <Label className="text-xs font-light font-display tracking-[0.08em] text-foreground">
                        Show Server in Server Browser
                      </Label>
                    </div>
                  </InstanceFormField>
                )}
              />
            </div>
            <Separator className="my-4" />
            <Button
              type="submit"
              variant="server"
              size="lg"
              className="h-16 w-full text-xl"
              disabled={isSubmitting}
            >
              {isSubmitting ? "Creating Instance..." : "Create Instance"}
              {isSubmitting ? (
                <Loader2
                  data-icon="inline-end"
                  className="size-4 animate-spin"
                />
              ) : (
                <Sparkles data-icon="inline-end" className="size-4" />
              )}
            </Button>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}
