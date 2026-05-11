import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { api } from "@/lib/api";
import { StatusBadge } from "@/components/StatusBadge";
import { formatRelativeTime } from "@/lib/utils";

export function EndpointsPage() {
  const { data: endpoints, isLoading, error, refetch } = useQuery({
    queryKey: ["endpoints"],
    queryFn: () => api.endpoints.list(),
    refetchInterval: 30_000,
  });

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-semibold">Endpoints</h2>
          <p className="text-sm text-muted-foreground mt-1">
            {endpoints?.length ?? 0} total
          </p>
        </div>
        <button
          onClick={() => refetch()}
          className="text-sm px-3 py-1.5 rounded-md border hover:bg-accent transition-colors"
        >
          Refresh
        </button>
      </div>

      {isLoading && <p className="text-sm text-muted-foreground">Loading…</p>}
      {error && (
        <p className="text-sm text-destructive">Error: {String(error)}</p>
      )}

      <div className="rounded-md border overflow-hidden">
        <table className="w-full text-sm">
          <thead className="bg-muted/50">
            <tr>
              <th className="text-left px-4 py-2.5 font-medium text-muted-foreground">Hostname</th>
              <th className="text-left px-4 py-2.5 font-medium text-muted-foreground">IP</th>
              <th className="text-left px-4 py-2.5 font-medium text-muted-foreground">OS</th>
              <th className="text-left px-4 py-2.5 font-medium text-muted-foreground">Status</th>
              <th className="text-left px-4 py-2.5 font-medium text-muted-foreground">Last seen</th>
            </tr>
          </thead>
          <tbody className="divide-y">
            {(endpoints ?? []).map((ep) => (
              <tr key={ep.id} className="hover:bg-accent/30 transition-colors">
                <td className="px-4 py-2.5">
                  <Link
                    to="/endpoints/$endpointId"
                    params={{ endpointId: ep.id }}
                    className="font-medium hover:underline"
                  >
                    {ep.network?.primary_hostname ?? ep.id}
                  </Link>
                </td>
                <td className="px-4 py-2.5 text-muted-foreground">
                  {ep.network?.primary_ip ?? "—"}
                </td>
                <td className="px-4 py-2.5 text-muted-foreground">
                  {ep.identity?.distro ?? "—"}
                  {ep.identity?.arch ? ` (${ep.identity.arch})` : ""}
                </td>
                <td className="px-4 py-2.5">
                  <StatusBadge status={ep.status} />
                </td>
                <td className="px-4 py-2.5 text-muted-foreground">
                  {formatRelativeTime(ep.agent?.last_seen)}
                </td>
              </tr>
            ))}
            {(endpoints?.length === 0) && !isLoading && (
              <tr>
                <td colSpan={5} className="px-4 py-8 text-center text-muted-foreground">
                  No endpoints found.
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}
