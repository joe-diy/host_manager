import { useQuery } from "@tanstack/react-query";
import { api, type EndpointStatus } from "@/lib/api";
import { StatusBadge } from "@/components/StatusBadge";
import { Link } from "@tanstack/react-router";

const STATUS_ORDER: EndpointStatus[] = [
  "managed",
  "identified",
  "discovered",
  "agent_deploying",
  "offline",
  "degraded",
  "decommissioned",
];

export function DashboardPage() {
  const { data: endpoints, isLoading, error } = useQuery({
    queryKey: ["endpoints"],
    queryFn: () => api.endpoints.list(),
    refetchInterval: 30_000,
  });

  const counts = STATUS_ORDER.reduce<Record<string, number>>((acc, s) => {
    acc[s] = endpoints?.filter((e) => e.status === s).length ?? 0;
    return acc;
  }, {});

  const total = endpoints?.length ?? 0;

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-semibold">Dashboard</h2>
        <p className="text-sm text-muted-foreground mt-1">
          Fleet overview — auto-refreshes every 30s
        </p>
      </div>

      {isLoading && <p className="text-muted-foreground text-sm">Loading…</p>}
      {error && (
        <p className="text-destructive text-sm">
          Failed to load endpoints: {String(error)}
        </p>
      )}

      {/* Summary cards */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        <SummaryCard label="Total" value={total} />
        <SummaryCard label="Managed" value={counts.managed ?? 0} intent="success" />
        <SummaryCard label="Offline" value={counts.offline ?? 0} intent="muted" />
        <SummaryCard label="Degraded" value={counts.degraded ?? 0} intent="warn" />
      </div>

      {/* Status breakdown */}
      <section>
        <h3 className="text-sm font-medium mb-3 text-muted-foreground uppercase tracking-wide">
          By status
        </h3>
        <div className="flex flex-wrap gap-2">
          {STATUS_ORDER.map((s) => (
            <Link key={s} to="/endpoints" search={{ status: s }}>
              <span className="flex items-center gap-1.5">
                <StatusBadge status={s} />
                <span className="text-sm text-muted-foreground">{counts[s]}</span>
              </span>
            </Link>
          ))}
        </div>
      </section>

      {/* Recent endpoints */}
      <section>
        <h3 className="text-sm font-medium mb-3 text-muted-foreground uppercase tracking-wide">
          Recent endpoints
        </h3>
        <div className="rounded-md border divide-y">
          {(endpoints ?? []).slice(0, 10).map((ep) => (
            <Link
              key={ep.id}
              to="/endpoints/$endpointId"
              params={{ endpointId: ep.id }}
              className="flex items-center justify-between px-4 py-2.5 hover:bg-accent transition-colors"
            >
              <div>
                <p className="text-sm font-medium">
                  {ep.network?.primary_hostname ?? ep.id}
                </p>
                <p className="text-xs text-muted-foreground">
                  {ep.network?.primary_ip ?? "—"}
                </p>
              </div>
              <StatusBadge status={ep.status} />
            </Link>
          ))}
          {total === 0 && !isLoading && (
            <p className="px-4 py-4 text-sm text-muted-foreground text-center">
              No endpoints yet.{" "}
              <Link to="/discovery" className="underline">
                Run a discovery scan
              </Link>{" "}
              to get started.
            </p>
          )}
        </div>
      </section>
    </div>
  );
}

function SummaryCard({
  label,
  value,
  intent,
}: {
  label: string;
  value: number;
  intent?: "success" | "warn" | "muted";
}) {
  const color =
    intent === "success"
      ? "text-green-600"
      : intent === "warn"
      ? "text-orange-500"
      : intent === "muted"
      ? "text-gray-400"
      : "text-foreground";
  return (
    <div className="rounded-lg border p-4">
      <p className="text-xs text-muted-foreground mb-1">{label}</p>
      <p className={`text-3xl font-bold ${color}`}>{value}</p>
    </div>
  );
}
