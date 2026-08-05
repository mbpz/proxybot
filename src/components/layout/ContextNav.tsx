import { NavLink } from "react-router-dom";

export interface ContextNavItem {
  path: string;
  label: string;
  end?: boolean;
}

interface ContextNavProps {
  label: string;
  items: readonly ContextNavItem[];
}

/** Secondary navigation that keeps related tools inside one product context. */
export function ContextNav({ label, items }: ContextNavProps) {
  return (
    <nav
      aria-label={label}
      className="mb-4 flex shrink-0 items-center gap-1 rounded-lg border border-border bg-surface-secondary p-1"
    >
      {items.map((item) => (
        <NavLink
          key={item.path}
          to={item.path}
          end={item.end}
          className={({ isActive }) =>
            `rounded-md px-3 py-1.5 text-sm transition-colors ${
              isActive
                ? "bg-surface-tertiary text-accent-blue"
                : "text-text-secondary hover:bg-surface-tertiary hover:text-text-primary"
            }`
          }
        >
          {item.label}
        </NavLink>
      ))}
    </nav>
  );
}
