import {
  DashboardOwnedInstancesCard,
  DashboardSettingsCard,
  DashboardSyncedInstancesCard,
} from "@/features/instances/components/instance-cards";

export function Dashboard() {
  return (
    <div className="mx-auto flex w-full max-w-3xl flex-col gap-4">
      <div className="grid gap-4 sm:grid-cols-2">
        <DashboardSyncedInstancesCard />
        <DashboardOwnedInstancesCard />
      </div>
      <div className="grid gap-4 sm:grid-cols-2 ">
        <div className="flex flex-col items-center  rounded-xl bg-card/50 p-4">
          <p className="text-base  tracking-wide text-foreground">
            {`Synced Instances are instances that you have synced by connecting to another Nerevar server. They are saved configurations to easily re-connect to that server and do not expose all the configuration and options as an owned instance, however they can be converted to an owned instance.`}
          </p>
        </div>
        <div className="flex flex-col items-center  rounded-xl bg-card/50 p-4">
          <p className="text-base tracking-wide text-foreground">
            {`Owned Instances are instances that you have manually created and configured to host a server. They contain full configuarion and options to tweak and manage data such as mods, settings, and more.`}
          </p>
        </div>
      </div>
      <DashboardSettingsCard className="col-span-2" />
    </div>
  );
}
