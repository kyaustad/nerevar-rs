import { useCallback, useEffect, useState } from "react";
import {
  applyTheme,
  getStoredTheme,
  getSystemTheme,
  resolveTheme,
  setStoredTheme,
  type Theme,
} from "@/lib/theme";

export function useTheme() {
  const [theme, setThemeState] = useState<Theme>(() => getStoredTheme());
  const [resolved, setResolved] = useState<"light" | "dark">(() =>
    resolveTheme(getStoredTheme()),
  );

  useEffect(() => {
    applyTheme(theme);
    setResolved(resolveTheme(theme));

    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const onChange = () => {
      if (theme === "system") {
        applyTheme("system");
        setResolved(getSystemTheme());
      }
    };

    media.addEventListener("change", onChange);
    return () => media.removeEventListener("change", onChange);
  }, [theme]);

  const setTheme = useCallback((next: Theme) => {
    setStoredTheme(next);
    setThemeState(next);
    applyTheme(next);
    setResolved(resolveTheme(next));
  }, []);

  const cycleTheme = useCallback(() => {
    const order: Theme[] = ["dark", "light", "system"];
    const index = order.indexOf(theme);
    setTheme(order[(index + 1) % order.length]);
  }, [theme, setTheme]);

  return {
    theme,
    resolved,
    setTheme,
    cycleTheme,
    isDark: resolved === "dark",
    systemTheme: getSystemTheme(),
  };
}
