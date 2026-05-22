import React from "react";
import ReactDOM from "react-dom/client";
import { ParticlesProvider } from "@tsparticles/react";
import App from "./App";
// import App from "./components/custom/default-showcase";
import "./App.css";
import { initParticlesEngine } from "./lib/particles-init";
import { initTheme } from "./lib/theme";

initTheme();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ParticlesProvider init={initParticlesEngine}>
      <App />
    </ParticlesProvider>
  </React.StrictMode>,
);
