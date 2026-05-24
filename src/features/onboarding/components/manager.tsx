/*
This component is responsible for managing the onboarding process.

It needs to:

- set the base Nerevar data directory (different from app data directory, this one will be used to store actual downloaded files such as tes3mp and instances and their data/configs)
- allow the user to select the sync port for the Nerevar server from default 25567
- display a guide on how creating instances works
- call tauri to mark the onboarding as complete

*/

import { NerevarBackgroundShell } from "@/components/custom/nerevar-background-shell";
import { Button } from "@/components/ui/button";
import { useConfig } from "@/features/config/context/config-context-provider";
import { EmberParticles } from "@/components/custom/ember-particles";

export function OnboardingManager({
  children,
  onComplete,
}: {
  children: React.ReactNode;
  onComplete: () => void;
}) {
  const config = useConfig();

  if (!config) {
    return (
      <NerevarBackgroundShell className="flex min-h-screen flex-col items-center justify-center gap-4 relative">
        <EmberParticles /> <p>Loading...</p>
      </NerevarBackgroundShell>
    );
  } else if (config.onboardingComplete) {
    return (
      <NerevarBackgroundShell className="flex min-h-screen flex-col items-center justify-center gap-4 relative">
        <EmberParticles />
        {children}
      </NerevarBackgroundShell>
    );
  } else {
    return (
      <NerevarBackgroundShell className="flex min-h-screen flex-col items-center justify-center gap-4 relative">
        <EmberParticles />
        <h1>Onboarding Manager</h1>
        <p>
          Config:{" "}
          {config.onboardingComplete
            ? "Onboarding Complete"
            : "Onboarding Incomplete"}
        </p>
        <p>Instances: {config.instances?.length}</p>
        <p>Root Instance Path: {config.rootInstancePath}</p>
        <p>Sync Port: {config.syncPort}</p>
        <Button onClick={onComplete}>Complete Onboarding</Button>
      </NerevarBackgroundShell>
    );
  }
}
