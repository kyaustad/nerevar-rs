import { Monitor, Moon, Sun } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { useTheme } from "@/hooks/use-theme";
import type { Theme } from "@/lib/theme";
import { cn } from "@/lib/utils";

const LABELS: Record<Theme, string> = {
  dark: "Dark mode",
  light: "Light mode",
  system: "System theme",
};

function ThemeIcon({ theme }: { theme: Theme }) {
  switch (theme) {
    case "light":
      return <Sun className="size-4.5" />;
    case "system":
      return <Monitor className="size-4.5" />;
    default:
      return <Moon className="size-4.5" />;
  }
}

export function ThemeSwitcher({ className }: { className?: string }) {
  const { theme, cycleTheme } = useTheme();

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button
          type="button"
          variant="outline"
          size="icon"
          className={cn(
            "text-accent hover:bg-accent/10 hover:text-accent",
            className,
          )}
          onClick={cycleTheme}
          aria-label={LABELS[theme]}
        >
          <ThemeIcon theme={theme} />
        </Button>
      </TooltipTrigger>
      <TooltipContent side="bottom">
        {LABELS[theme]} — click to switch
      </TooltipContent>
    </Tooltip>
  );
}
