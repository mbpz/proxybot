type BadgeVariant =
  | "get"
  | "post"
  | "put"
  | "delete"
  | "patch"
  | "direct"
  | "proxy"
  | "reject"
  | "info"
  | "warning"
  | "critical"
  | "wechat"
  | "douyin"
  | "alipay"
  | "unknown";

interface BadgeProps {
  variant: BadgeVariant;
  children: React.ReactNode;
  className?: string;
}

export function Badge({ variant, children, className = "" }: BadgeProps) {
  return (
    <span className={`badge badge-${variant} ${className}`}>{children}</span>
  );
}

interface MethodBadgeProps {
  method: string;
}

export function MethodBadge({ method }: MethodBadgeProps) {
  const variant = method.toLowerCase() as BadgeVariant;
  const validVariants: BadgeVariant[] = [
    "get",
    "post",
    "put",
    "delete",
    "patch",
  ];
  const badgeVariant = validVariants.includes(variant) ? variant : "info";

  return <Badge variant={badgeVariant}>{method}</Badge>;
}

interface AppBadgeProps {
  app: string | null;
}

export function AppBadge({ app }: AppBadgeProps) {
  if (!app) return <Badge variant="unknown">Unknown</Badge>;

  const variant = app.toLowerCase() as BadgeVariant;
  const validVariants: BadgeVariant[] = ["wechat", "douyin", "alipay"];
  const badgeVariant = validVariants.includes(variant) ? variant : "unknown";

  return <Badge variant={badgeVariant}>{app}</Badge>;
}
