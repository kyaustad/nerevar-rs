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

// TES3MP-NG releases (kyaustad/TES3MP-NG), newest first
function releaseLabel(release: GithubReleaseResponse): string {
  return release.name || `TES3MP-NG ${release.tag_name}`;
}

/** The newest release that isn't a pre-release. */
function recommendedRelease(
  releases: GithubReleaseResponse[],
): GithubReleaseResponse | undefined {
  return releases.find((release) => !release.prerelease) ?? releases[0];
}

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

  const recommended = recommendedRelease(releases);
  const showRecommendedBadge =
    recommended !== undefined && recommended.id.toString() === value;

  useEffect(() => {
    const fetchReleases = async () => {
      const releases =
        await invoke<GithubReleaseResponse[]>("get_all_releases");
      setReleases(releases || []);
    };
    fetchReleases();
  }, []);

  // Preselect the recommended release
  useEffect(() => {
    if (!value && recommended) {
      onValueChange(recommended.id.toString());
    }
  }, [value, recommended, onValueChange]);

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
            key={release.id}
            value={release.id.toString()}
            textValue={releaseLabel(release)}
            className="my-1 justify-between rounded-lg border border-border/50 p-2"
          >
            <SelectItemText>{releaseLabel(release)}</SelectItemText>
            {release.id === recommended?.id && (
              <RecommendedBadge className="mr-8" />
            )}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}
