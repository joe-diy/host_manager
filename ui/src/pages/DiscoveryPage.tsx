import { useState } from "react";
import { useMutation, useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";
import { formatRelativeTime } from "@/lib/utils";

export function DiscoveryPage() {
  const [subnet, setSubnet] = useState("");

  const { data: status, refetch: refetchStatus } = useQuery({
    queryKey: ["discovery-status"],
    queryFn: () => api.discovery.status(),
    refetchInterval: 10_000,
  });

  const startMutation = useMutation({
    mutationFn: () => api.discovery.start(subnet || undefined),
    onSuccess: () => {
      setSubnet("");
      refetchStatus();
    },
  });

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-semibold">Discovery</h2>
        <p className="text-sm text-muted-foreground mt-1">
          Scan your network for new endpoints
        </p>
      </div>

      {/* Last run summary */}
      <div className="rounded-lg border p-4 space-y-1">
        <p className="text-sm font-medium">Last run</p>
        <div className="grid grid-cols-3 gap-4 mt-2">
          <div>
            <p className="text-xs text-muted-foreground">Completed</p>
            <p className="text-sm">{formatRelativeTime(status?.completed_at)}</p>
          </div>
          <div>
            <p className="text-xs text-muted-foreground">Subnet</p>
            <p className="text-sm font-mono">{status?.subnet ?? "—"}</p>
          </div>
          <div>
            <p className="text-xs text-muted-foreground">Found</p>
            <p className="text-sm">{status?.count ?? "—"}</p>
          </div>
        </div>
      </div>

      {/* Start new run */}
      <div className="rounded-lg border p-4 space-y-3">
        <p className="text-sm font-medium">Start a new scan</p>
        <div className="flex gap-2 items-center">
          <input
            type="text"
            value={subnet}
            onChange={(e) => setSubnet(e.target.value)}
            placeholder="Subnet CIDR (e.g. 192.168.1.0/24) — leave blank for default"
            className="flex-1 rounded-md border px-3 py-1.5 text-sm bg-background"
          />
          <button
            onClick={() => startMutation.mutate()}
            disabled={startMutation.isPending}
            className="px-4 py-1.5 rounded-md bg-primary text-primary-foreground text-sm disabled:opacity-50 shrink-0"
          >
            {startMutation.isPending ? "Starting…" : "Start scan"}
          </button>
        </div>
        {startMutation.isSuccess && (
          <p className="text-xs text-green-600">Scan started successfully.</p>
        )}
        {startMutation.isError && (
          <p className="text-xs text-destructive">
            {String(startMutation.error)}
          </p>
        )}
      </div>
    </div>
  );
}
