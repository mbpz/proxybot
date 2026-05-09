import { useState } from "react";
import { generateCurl, generateFetch, generatePython, generateGo } from "./codeGenerators";

interface CodeExportProps {
  method: string;
  url: string;
  headers: Record<string, string>;
  body?: string;
}

type ExportFormat = "curl" | "fetch" | "python" | "go";

const FORMAT_LABELS: Record<ExportFormat, string> = {
  curl: "cURL",
  fetch: "fetch()",
  python: "Python requests",
  go: "Go http",
};

const GENERATORS: Record<ExportFormat, (m: string, u: string, h: Record<string, string>, b: string) => string> = {
  curl: generateCurl,
  fetch: generateFetch,
  python: generatePython,
  go: generateGo,
};

export function CodeExport({ method, url, headers, body = "" }: CodeExportProps) {
  const [copied, setCopied] = useState(false);

  function handleCopy(format: ExportFormat) {
    const code = GENERATORS[format](method, url, headers, body);
    navigator.clipboard.writeText(code).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }).catch((err) => {
      console.error("Failed to copy:", err);
    });
  }

  return (
    <div className="relative inline-block">
      <div className="flex gap-1">
        {(Object.keys(FORMAT_LABELS) as ExportFormat[]).map((fmt) => (
          <button
            key={fmt}
            onClick={() => handleCopy(fmt)}
            className="px-2 py-1 text-xs bg-gray-100 hover:bg-gray-200 rounded transition-colors"
            title={`Copy as ${FORMAT_LABELS[fmt]}`}
          >
            {FORMAT_LABELS[fmt]}
          </button>
        ))}
      </div>
      {copied && (
        <span className="absolute -top-6 left-0 text-xs text-green-600">Copied!</span>
      )}
    </div>
  );
}
