import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Separator } from "@/components/ui/separator";
import { Textarea } from "@/components/ui/textarea";
import { ReleaseSelector } from "@/features/tes3mp-releases/components/release-selector";
import type { GithubReleaseResponse } from "@/types";
import { ArrowLeft, FolderOpen } from "lucide-react";
import { useState } from "react";
import { navigate } from "wouter/use-browser-location";

// Create a new instance steps
// 1. Select TES3MP release
// 2. Enter instance name
// 3. Enter instance description
// 4. Select instance root path
// 5. Define tes3mp server defaults like password, server name, port, post to master server, etc
// These values are ephemeral and passed into the rust function to modify the tes3mp-server-defaults.cfg file after unzipping the release

export function NewInstancePage() {
  const [selectedRelease, setSelectedRelease] =
    useState<GithubReleaseResponse | null>(null);

  const [instanceName, setInstanceName] = useState<string>("");
  const [instanceDescription, setInstanceDescription] = useState<string>("");
  const [instanceRootPath, setInstanceRootPath] = useState<string>("");

  const handleInstanceNameChange = (name: string) => {
    setInstanceName(name);
  };
  const handleInstanceDescriptionChange = (description: string) => {
    setInstanceDescription(description);
  };
  const handleInstanceRootPathChange = (path: string) => {
    setInstanceRootPath(path);
  };

  const handleReleaseChange = (release: GithubReleaseResponse) => {
    setSelectedRelease(release);
    console.log(release);
  };
  return (
    <div className="mx-auto flex w-full max-w-4xl flex-col items-center gap-6 py-8 text-center">
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

      <Card className="w-full max-w-lg" disableHover disableTap>
        <CardContent>
          <div className="flex flex-col gap-2">
            <Label className="text-lg font-light font-display tracking-[0.08em] text-foreground">
              TES3MP Release
            </Label>
            <p className="text-xs text-foreground/75 font-light tracking-[0.1em] font-sans text-left leading-loose">
              {`This is the version of TES3MP that will be used for this instance. The latest non-VR release TES3MP 0.8.1 is recommended.`}
            </p>
            <ReleaseSelector onReleaseChange={handleReleaseChange} />
          </div>
          <Separator className="my-4" />
          <div className="flex flex-col gap-2">
            <Label className="text-lg font-light font-display tracking-[0.08em] text-foreground">
              Instance Name
            </Label>
            <p className="text-xs text-foreground/75 font-light tracking-[0.1em] font-sans text-left leading-loose">
              {`This is the name of the instance. It will be used to identify the instance in the UI and in the file system.`}
            </p>
            <Input
              value={instanceName}
              onChange={(e) => handleInstanceNameChange(e.target.value)}
            />
          </div>
          <Separator className="my-4" />
          <div className="flex flex-col gap-2">
            <Label className="text-lg font-light font-display tracking-[0.08em] text-foreground">
              Instance Description
            </Label>
            <p className="text-xs text-foreground/75 font-light tracking-[0.1em] font-sans text-left leading-loose">
              {`This is the description of the instance. It will be used to provide more information about the instance.`}
            </p>
            <Textarea
              value={instanceDescription}
              onChange={(e) => handleInstanceDescriptionChange(e.target.value)}
            />
          </div>
          <Separator className="my-4" />
          <div className="flex flex-col gap-2">
            <Label className="text-lg font-light font-display tracking-[0.08em] text-foreground">
              Instance Root Path
            </Label>
            <p className="text-xs text-foreground/75 font-light tracking-[0.1em] font-sans text-left leading-loose">
              {`This is the root path of the instance. It will be used to store the instance data.`}
            </p>
            <div className="flex flex-row gap-4">
              <Input
                readOnly
                value={instanceRootPath}
                onChange={(e) => handleInstanceRootPathChange(e.target.value)}
              />
              <Button
                variant="outline"
                className="shrink-0 font-display text-[0.75rem] tracking-[0.3em] uppercase"
              >
                <FolderOpen className="size-4" />
                Browse
              </Button>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
