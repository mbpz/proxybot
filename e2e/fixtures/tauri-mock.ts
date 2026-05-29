import type { Page } from "@playwright/test";

/**
 * Inject Tauri IPC mock into the page before the app loads.
 * Sets up window.__TAURI_INTERNALS__ so invoke() routes to the handler.
 * The handler is injected as a function body string to avoid serialization issues.
 */
export async function mockTauriIPC(page: Page, handlerFn: string): Promise<void> {
  await page.addInitScript((fnBody) => {
    const cbMap = new Map<number, (data: unknown) => void>();
    let nextId = 1;

    function transformCallback(callback: (data: unknown) => void, once = false): number {
      const id = nextId++;
      cbMap.set(id, (data) => {
        if (once) cbMap.delete(id);
        callback(data);
      });
      return id;
    }

    function unregisterCallback(id: number): void {
      cbMap.delete(id);
    }

    function runCallback(id: number, data: unknown): void {
      cbMap.get(id)?.(data);
    }

    // Build handler from string body
    const handler = new Function("cmd", "args", fnBody) as (cmd: string, args?: Record<string, unknown>) => unknown;

    window.__TAURI_INTERNALS__ = {
      invoke: (cmd: string, args?: Record<string, unknown>) => {
        return Promise.resolve(handler(cmd, args));
      },
      transformCallback,
      unregisterCallback,
      runCallback,
      callbacks: cbMap,
    };

    window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {
      unregisterListener: () => {},
    };
  }, handlerFn);
}

/**
 * Convenience: mock a set of invoke commands with static return values.
 */
export async function mockTauriCommands(
  page: Page,
  mocks: Record<string, unknown>,
): Promise<void> {
  // Build a function body that does a lookup
  const entries = Object.entries(mocks).map(([k, v]) => [k, JSON.stringify(v)] as [string, string]);
  const switchCases = entries.map(([k, v]) => `case ${JSON.stringify(k)}: return ${v};`).join("\n");
  const fnBody = `switch(cmd) {\n${switchCases}\ndefault: return null;\n}`;
  await mockTauriIPC(page, fnBody);
}
