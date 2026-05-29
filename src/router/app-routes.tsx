import { AppLayout } from "@/app/app-layout";
import { InstanceDataManagerPage } from "@/features/instances/pages/instance-data-manager";
import { InstanceDetailPage } from "@/features/instances/pages/instance-detail";
import {
  OwnedInstancesOverview,
  SyncedInstancesOverview,
} from "@/features/instances/components/overview";
import { Link, Route, Router, Switch, useLocation } from "wouter";
import { NewInstancePage } from "@/features/instances/pages/new-owned-instance";
import { NewConnectionPage } from "@/features/instances/pages/new-connection";
import { Dashboard } from "@/features/dashboard/components/dashboard";
import { useEffect, useState } from "react";

const SUBTITLE_OPTIONS = [
  "What do you want, outlander?",
  "Wealth beyond measure outlander.",
  "You like to dance close to the fire, don't you?",
  "Why walk when you can ride?",

  "I've faught guars more ferocious than you!",
  "The prey approaches...",
  "Tell your friends about this place.",
];

const getRandomSubtitle = (currentSubtitle: string) => {
  return SUBTITLE_OPTIONS.filter((subtitle) => subtitle !== currentSubtitle)[
    Math.floor(Math.random() * SUBTITLE_OPTIONS.length)
  ];
};

function NotFound() {
  return (
    <div className="flex flex-col items-center gap-4 py-16 text-center">
      <p className="font-display text-sm tracking-[0.2em] text-foreground/60 uppercase">
        Page not found
      </p>
      <Link
        href="/"
        className="font-display text-xs tracking-[0.2em] text-accent uppercase hover:underline"
      >
        Return home
      </Link>
    </div>
  );
}

export function AppRoutes() {
  const [subtitle, setSubtitle] = useState(
    getRandomSubtitle(SUBTITLE_OPTIONS[2]),
  );
  const [pathname] = useLocation();

  useEffect(() => {
    setSubtitle(getRandomSubtitle(subtitle));
  }, [pathname]);
  return (
    <Router>
      <AppLayout subtitle={subtitle}>
        <Switch>
          <Route path="/" component={Dashboard} />
          <Route path="/owned-instances" component={OwnedInstancesOverview} />
          <Route path="/synced-instances" component={SyncedInstancesOverview} />
          <Route path="/new-instance" component={NewInstancePage} />
          <Route path="/new-connection" component={NewConnectionPage} />
          <Route path="/instances/:id/data" component={InstanceDataManagerPage} />
          <Route path="/instances/:id" component={InstanceDetailPage} />
          <Route component={NotFound} />
        </Switch>
      </AppLayout>
    </Router>
  );
}
