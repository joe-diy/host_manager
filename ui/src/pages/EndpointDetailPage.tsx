import { useQuery, useMutation } from "@tanstack/react-query";
import { useParams } from "@tanstack/react-router";
import { api } from "@/lib/api";
import { StatusBadge } from "@/components/StatusBadge";
import { formatRelativeTime } from "@/lib/utils";
import { useState } from "react";

export function EndpointDetailPage() {
  const { endpointId } = useParams({ from: "/endpoints/$endpointId" });
  const [cmd, setCmd] = useState("");
  const [cmdResult, setCmdResult] = useState<string | null>(null);

  const { data: ep, isLoading, error } = useQuery({
    queryKey: ["endpoint", endpointId],
    queryFn: () => api.endpoints.get(endpointId),
    refetchInterval: 30_000,
  });

  const dispatchMutation = useMutation({
    mutationFn: (command: string) =>
      api.endpoints.sendCommand(endpointId, {
        type: "exec",
        command,
        timeout_seconds: 30,
      }),
    onSuccess: (data) => {
      setCmdResult(`Command dispatched — ID: ${data.command_id}`);
      setCmd("");
    },
    onError: (err) => setCmdResult(`Error: ${String(err)}`),
  });

  if (isLoading) return <p className="text-sm text-muted-foreground">Loading…</p>;
  if (error || !ep) return <p className="text-sm text-destructive">Endpoint not found.</p>;

  return (
    <div className="space-y-6">
      <div className="flex items-start justify-between">
        <div>
          <h2 className="text-2xl font-semibold">
            {ep.network?.primary_hostname ?? ep.id}
          </h2>
          <p className="text-sm text-muted-foreground mt-1 font-mono">{ep.id}</p>
        </div>
        <StatusBadge status={ep.status} className="mt-1" />
      </div>

      {/* Details grid */}
      <div className="grid grid-cols-2 md:grid-cols-3 gap-4">
        <Detail label="IP address" value={ep.network?.primary_ip} />
        <Detail label="MAC address" value={ep.network?.mac_address} />
        <Detail label="OS" value={ep.identity?.distro} />
        <Detail label="Version" value={ep.identity?.version} />
        <Detail label="Architecture" value={ep.identity?.arch} />
        <Detail label="Agent version" value={ep.agent?.version} />
        <Detail label="Transport" value={ep.agent?.transport} />
        <Detail label="Last seen" value={formatRelativeTime(ep.agent?.last_seen)} />
        <Detail label="Discovered" value={formatRelativeTime(ep.created_at)} />
      </div>

      {/* Tags */}
      {Object.keys(ep.tags ?? {}).length > 0 && (
        <section>
          <h3 className="text-sm font-medium mb-2 text-muted-foreground">Tags</h3>
          <div className="flex flex-wrap gap-2">
            {Object.entries(ep.tags).map(([k, v]) => (
              <span key={k} className="text-xs px-2 py-1 rounded-full bg-muted font-mono">
                {k}={v}
              </span>
            ))}
          </div>
        </section>
      )}

      {/* Command dispatch (only for managed endpoints) */}
      {ep.status === "managed" && (
        <section className="space-y-2">
          <h3 className="text-sm font-medium text-muted-foreground">Run command</h3>
          <div className="flex gap-2">
            <input
              type="text"
              value={cmd}
              onChange={(e) => setCmd(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && cmd && dispatchMutation.mutate(cmd)}
              placeholder="e.g. uptime"
              className="flex-1 rounded-md border px-3 py-1.5 text-sm bg-background"
            />
            <button
              onClick={() => cmd && dispatchMutation.mutate(cmd)}
              disabled={!cmd || dispatchMutation.isPending}
              className="px-4 py-1.5 rounded-md bg-primary text-primary-foreground text-sm disabled:opacity-50"
            >
              {dispatchMutation.isPending ? "Sending…" : "Send"}
            </button>
          </div>
          {cmdResult && (
            <p className="text-xs text-muted-foreground font-mono bg-muted px-3 py-2 rounded">
              {cmdResult}
            </p>
          )}
        </section>
      )}
    </div>
  );
}

function Detail({ label, value }: { label: string; value?: string }) {
  return (
    <div>
      <p className="text-xs text-muted-foreground">{label}</p>
      <p className="text-sm font-medium mt-0.5">{value ?? "—"}</p>
    </div>
  );
}
