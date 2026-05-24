/*
This component is responsible for managing the onboarding process.

It needs to:

- set the base Nerevar data directory (different from app data directory, this one will be used to store actual downloaded files such as tes3mp and instances and their data/configs)
- allow the user to select the sync port for the Nerevar server from default 25567
- display a guide on how creating instances works
- call tauri to mark the onboarding as complete

*/

import { NerevarBackgroundShell } from "@/components/custom/nerevar-background-shell";
import { useConfig } from "@/features/config/context/config-context-provider";
import {
  OnboardingFlow,
  type OnboardingStage,
} from "@/features/onboarding/components/onboarding-flow";
import { useState } from "react";

export function OnboardingManager({
  children,
  onComplete,
}: {
  children: React.ReactNode;
  onComplete: () => void;
}) {
  const config = useConfig();
  const [stage, setStage] = useState<OnboardingStage>("select-data-dir");

  if (!config) {
    return (
      <NerevarBackgroundShell className="flex h-full min-h-full flex-col items-center justify-center">
        <p className="font-display text-xs tracking-[0.25em] text-foreground/60 uppercase animate-pulse">
          Loading...
        </p>
      </NerevarBackgroundShell>
    );
  }

  if (config.onboardingComplete) {
    return (
      <NerevarBackgroundShell className="min-h-full">
        {children}
      </NerevarBackgroundShell>
    );
  }

  return (
    <NerevarBackgroundShell className="h-full min-h-full">
      <OnboardingFlow
        stage={stage}
        onStageChange={setStage}
        onFinish={onComplete}
      />
    </NerevarBackgroundShell>
  );
}
