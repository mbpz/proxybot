// Shared types for the Filter DSL components.

export type FilterOp = "Eq" | "Glob" | "Regex" | "Gt" | "Lt" | "Gte" | "Lte";

export interface FilterPreset {
  id: string;
  name: string;
  expr: string;
}

export interface ParseResult {
  ok: boolean;
  error?: string;
}