import { useEffect, useRef, useMemo } from "react";
import mermaid from "mermaid";
import { GraphData } from "./types";

interface AuthStateMachineProps {
  data?: GraphData | null;
}

mermaid.initialize({
  startOnLoad: false,
  theme: "neutral",
});

const authKeywords = ["login", "auth", "token", "oauth", "signin", "password", "session"];

function buildMermaidDiagram(data?: GraphData | null): string {
  if (!data?.requests) {
    return "stateDiagram-v2\n  [*] --> NoAuthFlow\n  NoAuthFlow --> [*]";
  }

  const authStates: string[] = [];
  let currentState = "Initial";

  for (const req of data.requests.slice(0, 20)) {
    const combined = `${req.host} ${req.path}`.toLowerCase();
    const isAuth = authKeywords.some((kw) => combined.includes(kw));

    if (isAuth) {
      let newState = "Auth";
      if (combined.includes("login")) newState = "Login";
      else if (combined.includes("token")) newState = "Token";
      else if (combined.includes("logout")) newState = "Logout";

      if (!authStates.includes(newState) || authStates[authStates.length - 1] !== newState) {
        authStates.push(newState);
      }
      currentState = newState;
    } else if (currentState !== "API" && currentState !== "Initial") {
      if (authStates[authStates.length - 1] !== "API") {
        authStates.push("API");
      }
    }
  }

  if (authStates.length === 0) {
    return "stateDiagram-v2\n  [*] --> NoAuthFlow\n  NoAuthFlow --> [*]";
  }

  const transitions = authStates.map((state, i) => {
    if (i === 0) return `  [*] --> ${state}`;
    return `  ${authStates[i - 1]} --> ${state}`;
  });

  transitions.push(`  ${authStates[authStates.length - 1]} --> [*]`);

  return `stateDiagram-v2\n  ${transitions.join("\n  ")}`;
}

export function AuthStateMachine({ data }: AuthStateMachineProps) {
  const containerRef = useRef<HTMLDivElement>(null);

  const diagram = useMemo(() => buildMermaidDiagram(data), [data]);

  useEffect(() => {
    if (!containerRef.current) return;

    mermaid.render("auth-graph", diagram).then(({ svg }) => {
      if (containerRef.current) {
        containerRef.current.innerHTML = svg;
      }
    });

    return () => {
      if (containerRef.current) {
        containerRef.current.innerHTML = "";
      }
    };
  }, [diagram]);

  return (
    <div className="w-full h-full overflow-auto p-4">
      <div ref={containerRef} className="flex justify-center" />
    </div>
  );
}