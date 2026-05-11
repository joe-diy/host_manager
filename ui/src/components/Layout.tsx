import { Link } from "@tanstack/react-router";
import { cn } from "@/lib/utils";
import type { ReactNode } from "react";

const navItems = [
  { to: "/", label: "Dashboard", icon: "⬛" },
  { to: "/endpoints", label: "Endpoints", icon: "🖥" },
  { to: "/discovery", label: "Discovery", icon: "🔍" },
];

export function Layout({ children }: { children: ReactNode }) {
  return (
    <div className="flex h-screen bg-background">
      {/* Sidebar */}
      <aside className="w-56 border-r flex flex-col py-4 px-3 gap-1 shrink-0">
        <div className="mb-4 px-2">
          <h1 className="text-lg font-semibold tracking-tight">Host Manager</h1>
          <p className="text-xs text-muted-foreground">Fleet Control Plane</p>
        </div>
        {navItems.map((item) => (
          <NavLink key={item.to} to={item.to} icon={item.icon}>
            {item.label}
          </NavLink>
        ))}
        <div className="mt-auto">
          <a
            href="/auth/logout"
            className="flex items-center gap-2 px-2 py-1.5 rounded-md text-sm text-muted-foreground hover:text-foreground hover:bg-accent transition-colors"
          >
            Sign out
          </a>
        </div>
      </aside>

      {/* Main content */}
      <main className="flex-1 overflow-auto">
        <div className="container max-w-6xl py-8">{children}</div>
      </main>
    </div>
  );
}

function NavLink({
  to,
  icon,
  children,
}: {
  to: string;
  icon: string;
  children: ReactNode;
}) {
  return (
    <Link
      to={to}
      className={cn(
        "flex items-center gap-2 px-2 py-1.5 rounded-md text-sm transition-colors",
        "hover:bg-accent hover:text-accent-foreground",
        "[&.active]:bg-accent [&.active]:text-accent-foreground [&.active]:font-medium"
      )}
    >
      <span>{icon}</span>
      {children}
    </Link>
  );
}
