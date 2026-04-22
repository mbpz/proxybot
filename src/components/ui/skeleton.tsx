interface SkeletonProps {
  rows?: number;
}

export function SkeletonTable({ rows = 5 }: SkeletonProps) {
  return (
    <div>
      {Array.from({ length: rows }).map((_, i) => (
        <div key={i} className="skeleton-row">
          <div className="skeleton skeleton-cell sm" />
          <div className="skeleton skeleton-cell md" />
          <div className="skeleton skeleton-cell flex" />
          <div className="skeleton skeleton-cell sm" />
          <div className="skeleton skeleton-cell sm" />
        </div>
      ))}
    </div>
  );
}

interface SkeletonCardProps {
  lines?: number;
}

export function SkeletonCard({ lines = 3 }: SkeletonCardProps) {
  return (
    <div className="card">
      <div className="skeleton skeleton-cell md" style={{ marginBottom: 12 }} />
      {Array.from({ length: lines }).map((_, i) => (
        <div key={i} className="skeleton skeleton-cell flex" style={{ marginBottom: 8 }} />
      ))}
    </div>
  );
}
