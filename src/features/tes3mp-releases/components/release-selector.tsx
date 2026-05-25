import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import type { GithubReleaseResponse } from "@/types";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectItemText,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";

const RECOMMENDED_RELEASE = "TES3MP 0.8.1";

function RecommendedBadge({ className }: { className?: string }) {
  return (
    <Badge
      variant="outline"
      className={cn(
        "ml-auto shrink-0 border-accent/40 bg-gradient-to-r from-accent/80 to-accent/40 px-2 font-bold tracking-[0.08em] text-gradient-gold glow-gold",
        className,
      )}
    >
      Recommended
    </Badge>
  );
}

export function ReleaseSelector({
  value,
  onValueChange,
}: {
  value: string;
  onValueChange: (value: string) => void;
}) {
  const [releases, setReleases] = useState<GithubReleaseResponse[]>([]);

  const selectedRelease = releases.find(
    (release) => release.id.toString() === value,
  );
  const showRecommendedBadge = selectedRelease?.name === RECOMMENDED_RELEASE;

  useEffect(() => {
    const fetchReleases = async () => {
      const releases =
        await invoke<GithubReleaseResponse[]>("get_all_releases");
      setReleases(releases || []);
    };
    fetchReleases();
  }, []);

  const handleReleaseChange = (nextValue: string) => {
    onValueChange(nextValue);
  };

  if (releases.length === 0) {
    return (
      <Select disabled>
        <SelectTrigger className="min-w-full">
          <SelectValue
            placeholder="Loading..."
            className="text-foreground/50 animate-pulse"
          />
        </SelectTrigger>
      </Select>
    );
  }
  return (
    <Select value={value} onValueChange={handleReleaseChange}>
      <SelectTrigger className="flex w-full min-w-full">
        <span className="flex min-w-0 flex-1 items-center justify-between gap-2 pr-1">
          <SelectValue placeholder="Select a release" className="truncate" />
          {showRecommendedBadge && <RecommendedBadge />}
        </span>
      </SelectTrigger>
      <SelectContent
        position="popper"
        className="max-h-[350px] w-[var(--radix-select-trigger-width)]"
      >
        {releases.map((release) => (
          <SelectItem
            disabled={release.name !== RECOMMENDED_RELEASE}
            key={release.id}
            value={release.id.toString()}
            textValue={release.name}
            className="my-1 justify-between rounded-lg border border-border/50 p-2"
          >
            <SelectItemText>{release.name}</SelectItemText>
            {release.name === RECOMMENDED_RELEASE && (
              <RecommendedBadge className="mr-8" />
            )}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}
