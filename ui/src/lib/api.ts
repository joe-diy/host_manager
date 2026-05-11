/**
 * Typed API client for the Host Manager REST API.
 *
 * All requests are same-origin (UI is served by the api-gateway actor),
 * so the HttpOnly session cookie is sent automatically — no JS token handling.
 */

export interface Endpoint {
  id: string;
  status: EndpointStatus;
  schema_version: number;
  created_at: string;
  updated_at: string;
  tags: Record<string, string>;
  network?: NetworkInfo;
  identity?: IdentityInfo;
  agent?: AgentInfo;
}

export type EndpointStatus =
  | "discovered"
  | "identified"
  | "agent_deploying"
  | "managed"
  | "offline"
  | "degraded"
  | "decommissioned";

export interface NetworkInfo {
  primary_ip: string;
  primary_hostname?: string;
  mac_address?: string;
}

export interface IdentityInfo {
  os_family?: string;
  distro?: string;
  version?: string;
  arch?: string;
}

export interface AgentInfo {
  version?: string;
  transport?: string;
  last_seen?: string;
}

export interface DiscoveryStatus {
  completed_at?: string;
  subnet?: string;
  count: number;
}

// ---------------------------------------------------------------------------

async function request<T>(
  path: string,
  options?: RequestInit
): Promise<T> {
  const response = await fetch(path, {
    credentials: "same-origin",   // sends HttpOnly cookie
    headers: { "Content-Type": "application/json" },
    ...options,
  });

  if (!response.ok) {
    const text = await response.text().catch(() => response.statusText);
    throw new Error(`${response.status}: ${text}`);
  }

  return response.json() as Promise<T>;
}

export const api = {
  endpoints: {
    list: (params?: { status?: string }) => {
      const qs = params?.status ? `?status=${params.status}` : "";
      return request<Endpoint[]>(`/api/v1/endpoints${qs}`);
    },
    get: (id: string) => request<Endpoint>(`/api/v1/endpoints/${id}`),
    sendCommand: (id: string, payload: object) =>
      request<{ command_id: string }>(`/api/v1/endpoints/${id}/commands`, {
        method: "POST",
        body: JSON.stringify(payload),
      }),
  },

  discovery: {
    start: (subnet?: string) =>
      request<{ status: string }>("/api/v1/discovery/start", {
        method: "POST",
        body: JSON.stringify({ subnet }),
      }),
    status: () => request<DiscoveryStatus>("/api/v1/discovery/status"),
  },

  health: () => request<{ status: string }>("/api/v1/health"),
};
