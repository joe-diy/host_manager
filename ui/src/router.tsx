import {
  createRouter,
  createRootRoute,
  createRoute,
  Outlet,
} from "@tanstack/react-router";
import { Layout } from "./components/Layout";
import { DashboardPage } from "./pages/DashboardPage";
import { EndpointsPage } from "./pages/EndpointsPage";
import { EndpointDetailPage } from "./pages/EndpointDetailPage";
import { DiscoveryPage } from "./pages/DiscoveryPage";
import { LoginPage } from "./pages/LoginPage";

// Root route — renders the shell layout with nav sidebar
const rootRoute = createRootRoute({
  component: () => (
    <Layout>
      <Outlet />
    </Layout>
  ),
});

const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/",
  component: DashboardPage,
});

const endpointsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/endpoints",
  component: EndpointsPage,
});

const endpointDetailRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/endpoints/$endpointId",
  component: EndpointDetailPage,
});

const discoveryRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/discovery",
  component: DiscoveryPage,
});

const loginRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/login",
  component: LoginPage,
});

const routeTree = rootRoute.addChildren([
  indexRoute,
  endpointsRoute,
  endpointDetailRoute,
  discoveryRoute,
  loginRoute,
]);

export const router = createRouter({ routeTree });

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
