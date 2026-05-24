import { AppLayout } from "@/app/app-layout";
import { InstanceDetailPage } from "@/features/instances/pages/instance-detail";
import { InstancesOverview } from "@/features/instances/components/overview";
import { Link, Route, Router, Switch } from "wouter";
import { NewInstancePage } from "@/features/instances/pages/new-instance";

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
  return (
    <Router>
      <AppLayout>
        <Switch>
          <Route path="/" component={InstancesOverview} />
          <Route path="/new-instance" component={NewInstancePage} />
          <Route path="/instances/:id" component={InstanceDetailPage} />
          <Route component={NotFound} />
        </Switch>
      </AppLayout>
    </Router>
  );
}
