import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { cn } from "@/lib/utils";
import { ArrowLeft, ArrowRight } from "lucide-react";

export function OnboardingStepCard({
  children,
  className,
}: {
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <Card
      className={cn(
        "gap-0 border-border/80 bg-card/75 py-0 shadow-[0_0_20px_hsl(var(--accent)/0.08)] ring-accent/15 backdrop-blur-sm",
        className,
      )}
    >
      {children}
    </Card>
  );
}

export function StepActions({
  onBack,
  onPrimary,
  primaryLabel,
  showBack = true,
  nextDisabled = false,
}: {
  onBack?: () => void;
  onPrimary: () => void;
  primaryLabel: string;
  showBack?: boolean;
  nextDisabled?: boolean;
}) {
  return (
    <div className="flex items-center justify-between gap-3 border-t border-border/50 px-6 py-4">
      {showBack && onBack ? (
        <Button
          variant="outline"
          size="sm"
          onClick={onBack}
          className="text-foreground/70"
        >
          <ArrowLeft data-icon="inline-start" />
          Back
        </Button>
      ) : (
        <span />
      )}
      <Button
        variant="launch"
        size="sm"
        onClick={onPrimary}
        disabled={nextDisabled}
      >
        {primaryLabel}
        <ArrowRight data-icon="inline-end" />
      </Button>
    </div>
  );
}
