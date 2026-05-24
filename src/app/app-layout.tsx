import { NerevarHeader } from "@/components/custom/nerevar-header";

export function AppLayout({ children }: { children: React.ReactNode }) {
  return (
    <div className="relative flex min-h-full flex-col px-4 py-6 md:px-6 mt-8">
      <NerevarHeader title="NEREVAR" subtitle="What do you want, outlander?" />
      <main className="mx-auto w-full max-w-5xl flex-1">{children}</main>
    </div>
  );
}
