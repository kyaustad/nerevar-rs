import { useMemo } from "react";
import Particles, { useParticlesProvider } from "@tsparticles/react";
import { getEmberParticleOptions } from "@/lib/ember-particles-config";
import { useTheme } from "@/hooks/use-theme";
import { cn } from "@/lib/utils";

export function EmberParticles({ className }: { className?: string }) {
  const { loaded } = useParticlesProvider();
  const { resolved } = useTheme();
  const isDark = resolved === "dark";

  const options = useMemo(() => getEmberParticleOptions(isDark), [isDark]);

  if (!loaded) {
    return null;
  }

  return (
    <Particles
      key={resolved}
      id={`nerevar-ember-${resolved}`}
      className={cn("pointer-events-none fixed inset-0 size-full ", className)}
      options={options}
    />
  );
}
