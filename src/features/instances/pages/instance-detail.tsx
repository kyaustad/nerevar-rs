import { Button } from "@/components/ui/button";
import { useConfig } from "@/features/config/context/config-context-provider";
import { InstanceConfig } from "@/types";
import { Link, useParams } from "wouter";

export function InstanceDetailPage() {
  const params = useParams<{ id: string }>();
  const id = params.id ?? "unknown";

  const instance = useConfig()?.ownedInstances?.find(
    (instance: InstanceConfig) => instance.id === id,
  );

  if (!instance) {
    return (
      <div className="flex flex-col items-center gap-6 py-8 text-center">
        <p className="font-display text-sm tracking-[0.15em] text-accent uppercase">
          Instance not found
        </p>
      </div>
    );
  }

  return (
    <div className="flex flex-col items-center gap-6 py-8 text-center">
      <p className="font-display text-sm tracking-[0.15em] text-accent uppercase">
        Instance
      </p>
      <h2 className="font-display text-2xl tracking-[0.08em] text-gradient-gold">
        {instance?.name}
      </h2>
      <p className="max-w-md font-serif text-sm leading-relaxed text-foreground/70">
        {instance.description || "No description"}
      </p>
      <Button variant="outline" asChild>
        <Link href="/">Back to instances</Link>
      </Button>
    </div>
  );
}
