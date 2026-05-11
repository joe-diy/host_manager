import { type ClassValue, clsx } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

export function formatRelativeTime(iso?: string): string {
  if (!iso) return "—";
  const diff = Date.now() - new Date(iso).getTime();
  const s = Math.floor(diff / 1000);
  if (s < 60) return `${s}s ago`;
  if (s < 3600) return `${Math.floor(s / 60)}m ago`;
  if (s < 86400) return `${Math.floor(s / 3600)}h ago`;
  return `${Math.floor(s / 86400)}d ago`;
}

export function statusColor(status: string): string {
  const map: Record<string, string> = {
    managed: "text-green-600 bg-green-50",
    identified: "text-blue-600 bg-blue-50",
    discovered: "text-yellow-600 bg-yellow-50",
    offline: "text-gray-500 bg-gray-100",
    degraded: "text-orange-600 bg-orange-50",
    agent_deploying: "text-purple-600 bg-purple-50",
    decommissioned: "text-red-400 bg-red-50",
  };
  return map[status] ?? "text-gray-600 bg-gray-50";
}
