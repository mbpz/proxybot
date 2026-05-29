import { useMemo } from "react";
import DOMPurify from "dompurify";
import hljs from "highlight.js/lib/core";
import json from "highlight.js/lib/languages/json";
import xml from "highlight.js/lib/languages/xml";
import javascript from "highlight.js/lib/languages/javascript";
import css from "highlight.js/lib/languages/css";
import "highlight.js/styles/github-dark.css";

hljs.registerLanguage("json", json);
hljs.registerLanguage("xml", xml);
hljs.registerLanguage("javascript", javascript);
hljs.registerLanguage("css", css);

interface CodeViewerProps {
  content: string;
  contentType?: string;
  maxHeight?: string;
}

function detectLanguage(contentType?: string, content?: string): string {
  if (contentType) {
    const ct = contentType.toLowerCase();
    if (ct.includes("json")) return "json";
    if (ct.includes("xml") || ct.includes("html")) return "xml";
    if (ct.includes("javascript")) return "javascript";
    if (ct.includes("css")) return "css";
  }
  const trimmed = content?.trim() || "";
  if (trimmed.startsWith("{") || trimmed.startsWith("[")) return "json";
  if (trimmed.startsWith("<")) return "xml";
  return "plaintext";
}

function formatContent(content: string, lang: string): string {
  if (lang === "json") {
    try {
      return JSON.stringify(JSON.parse(content), null, 2);
    } catch {
      return content;
    }
  }
  return content;
}

export function CodeViewer({
  content,
  contentType,
  maxHeight = "32rem",
}: CodeViewerProps) {
  const lang = useMemo(
    () => detectLanguage(contentType, content),
    [contentType, content],
  );
  const formatted = useMemo(
    () => formatContent(content, lang),
    [content, lang],
  );

  const highlighted = useMemo(() => {
    if (lang === "plaintext" || !content) return null;
    try {
      const result = hljs.highlight(formatted, { language: lang });
      return result.value;
    } catch {
      return null;
    }
  }, [formatted, lang]);

  const sanitizedHtml = useMemo(() => {
    if (!highlighted) return null;
    return DOMPurify.sanitize(highlighted, {
      ALLOWED_TAGS: ["span", "br"],
      ALLOWED_ATTR: ["class"],
    });
  }, [highlighted]);

  if (!content) {
    return <div className="p-4 text-text-muted">No content</div>;
  }

  const lines = formatted.split("\n");

  return (
    <div className="relative overflow-auto" style={{ maxHeight }}>
      <div className="flex">
        {/* Line numbers */}
        <div className="select-none text-right pr-3 py-3 bg-surface-tertiary text-text-muted text-xs font-mono border-r border-border sticky left-0">
          {lines.map((_, i) => (
            <div key={i} className="leading-6" style={{ minWidth: "3ch" }}>
              {i + 1}
            </div>
          ))}
        </div>
        {/* Code */}
        <div className="flex-1 py-3 px-4 overflow-auto">
          {highlighted ? (
            <pre className="text-sm font-mono leading-6">
              <code
                className={`hljs language-${lang}`}
                dangerouslySetInnerHTML={{ __html: sanitizedHtml || "" }}
              />
            </pre>
          ) : (
            <pre className="text-sm font-mono leading-6 whitespace-pre-wrap">
              {formatted}
            </pre>
          )}
        </div>
      </div>
      {/* Language badge */}
      <div className="absolute top-2 right-2">
        <span className="badge badge-unknown">
          {lang}
        </span>
      </div>
    </div>
  );
}
