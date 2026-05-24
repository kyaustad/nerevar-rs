import { EmberParticles } from "./components/custom/ember-particles";
import { Button } from "./components/ui/button";
import { invoke } from "@tauri-apps/api/core";
import type { GithubReleaseResponse } from "./types";
import { useState, useEffect } from "react";
import type { NerevarConfig } from "./types";
import { OnboardingManager } from "./features/onboarding/components/manager";
import DefaultShowcase from "@/components/custom/default-showcase";

export default function App() {
  return (
    <div className="relative flex flex-col items-center justify-center gap-8 min-h-screen  text-foreground">
      <h1>Nerevar App</h1>
    </div>
  );
}
