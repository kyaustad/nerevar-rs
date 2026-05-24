import { EmberParticles } from "@/components/custom/ember-particles";
import { cn } from "@/lib/utils";

export function NerevarBackgroundShell({
  children,
  className,
}: {
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <div className="relative min-h-screen overflow-hidden nerevar-bg text-foreground">
      <div className="pointer-events-none absolute inset-0 z-0" aria-hidden>
        <div className="ember-fire-glow absolute inset-0" />
        <EmberParticles />
        <div className="absolute inset-0 bg-gradient-to-b from-background/25 via-transparent to-background/70" />
      </div>

      <div className={cn("relative z-10", className)}>{children}</div>
    </div>
  );
}
