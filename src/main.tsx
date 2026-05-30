import React from "react";
import ReactDOM from "react-dom/client";
import { ParticlesProvider } from "@tsparticles/react";
import App from "./App";
// import App from "./components/custom/default-showcase";
import "./App.css";
import { AppShell } from "@/app/app-shell";
import { initParticlesEngine } from "./lib/particles-init";
import { initTheme } from "./lib/theme";
import { Toaster } from "@/components/ui/sonner";
import { ConfigContextProvider } from "@/features/config/context/config-context-provider";
import { OnboardingManager } from "./features/onboarding/components/manager";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";

initTheme();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ConfigContextProvider>
      <AppShell>
        <ParticlesProvider init={initParticlesEngine}>
          <Toaster richColors />
          <OnboardingManager
            onComplete={() => {
              invoke("complete_onboarding");
              toast.success("Onboarding completed");
            }}
          >
            <App />
          </OnboardingManager>
        </ParticlesProvider>
      </AppShell>
    </ConfigContextProvider>
  </React.StrictMode>,
);
