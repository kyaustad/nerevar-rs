import { EmberParticles } from "./components/custom/ember-particles";
import { Button } from "./components/ui/button";
import { invoke } from "@tauri-apps/api/core";
import type { GithubReleaseResponse } from "./types";
import { useState } from "react";

export default function App() {
  const [releases, setReleases] = useState<GithubReleaseResponse[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [configDirectory, setConfigDirectory] = useState<string>("");

  const handleCreateConfigDirectory = async () => {
    try {
      const result = await invoke<string>("create_config_directory");
      setConfigDirectory(result);
    } catch (error) {
      console.error(error);
    }
  };

  const handleGetNerevarConfigDirectory = async () => {
    try {
      const result = await invoke<string>("get_nerevar_config_directory");
      setConfigDirectory(result);
    } catch (error) {
      console.error(error);
    }
  };

  const handleGetAllReleases = async () => {
    setIsLoading(true);
    try {
      const releases: GithubReleaseResponse[] =
        await invoke<GithubReleaseResponse[]>("get_all_releases");
      setReleases(releases);
    } catch (error) {
      console.error(error);
    } finally {
      setIsLoading(false);
    }
  };
  return (
    <div className="relative flex flex-col items-center justify-center gap-8 min-h-screen nerevar-bg text-foreground">
      <EmberParticles />
      <Button
        onClick={handleGetAllReleases}
        className="hover:scale-105 transition-all duration-300"
      >
        {isLoading ? "Loading..." : "Get All Releases"}
      </Button>
      <Button onClick={() => setReleases([])}>Clear Releases</Button>
      <Button onClick={handleCreateConfigDirectory}>
        Create Config Directory
      </Button>
      <Button onClick={handleGetNerevarConfigDirectory}>
        Get Nerevar Config Directory
      </Button>
      <p>Config Directory: {configDirectory}</p>
      <div className="flex flex-col gap-2 p-16">
        {releases.map((release) => (
          <div key={release.id}>
            <h3>{release.name}</h3>
            <p>{release.body}</p>
          </div>
        ))}
      </div>
    </div>
  );
}
