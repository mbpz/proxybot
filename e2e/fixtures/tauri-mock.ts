import type { Page } from "@playwright/test";

/**
 * Inject Tauri IPC mock into the page before the app loads.
 * Sets up window.__TAURI_INTERNALS__ so invoke() routes to the handler.
 * The handler is injected as a function body string to avoid serialization issues.
 */
export async function mockTauriIPC(page: Page, handlerFn: string): Promise<void> {
  await page.addInitScript((fnBody) => {
    const cbMap = new Map<number, (data: unknown) => void>();
    const eventListeners = new Map<string, Set<number>>();
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

    function handleEventPlugin(cmd: string, args?: Record<string, unknown>): unknown {
      const event = String(args?.event ?? "");
      if (cmd === "plugin:event|listen") {
        const handlerId = Number(args?.handler);
        const listeners = eventListeners.get(event) ?? new Set<number>();
        listeners.add(handlerId);
        eventListeners.set(event, listeners);
        return handlerId;
      }
      if (cmd === "plugin:event|unlisten") {
        const eventId = Number(args?.eventId);
        eventListeners.get(event)?.delete(eventId);
        unregisterCallback(eventId);
        return null;
      }
      throw new Error(`Unhandled Tauri event command: ${cmd}`);
    }

    function emitEvent(event: string, payload: unknown): void {
      for (const eventId of eventListeners.get(event) ?? []) {
        runCallback(eventId, { event, id: eventId, payload });
      }
    }

    window.__TAURI_INTERNALS__ = {
      invoke: (cmd: string, args?: Record<string, unknown>) => {
        try {
          if (cmd.startsWith("plugin:event|")) {
            return Promise.resolve(handleEventPlugin(cmd, args));
          }
          return Promise.resolve(handler(cmd, args));
        } catch (error) {
          return Promise.reject(error);
        }
      },
      transformCallback,
      unregisterCallback,
      runCallback,
      callbacks: cbMap,
      emitEvent,
    };

    window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {
      unregisterListener: (event: string, eventId: number) => {
        eventListeners.get(event)?.delete(eventId);
        unregisterCallback(eventId);
      },
    };
  }, handlerFn);
}

/** Emit through the same event routing used by Tauri's event plugin. */
export async function emitTauriEvent<T>(
  page: Page,
  event: string,
  payload: T,
): Promise<void> {
  await page.evaluate(
    ({ event, payload }) => {
      const internals = window.__TAURI_INTERNALS__ as typeof window.__TAURI_INTERNALS__ & {
        emitEvent?: (event: string, payload: unknown) => void;
      };
      internals.emitEvent?.(event, payload);
    },
    { event, payload },
  );
}

/**
 * Convenience: mock a set of invoke commands with static return values.
 */
export async function mockTauriCommands(
  page: Page,
  mocks: Record<string, unknown>,
): Promise<void> {
  const commands = { get_proxy_status: false, ...mocks };
  // Build a function body that does a lookup
  const entries = Object.entries(commands).map(([k, v]) => [k, JSON.stringify(v)] as [string, string]);
  const switchCases = entries.map(([k, v]) => `case ${JSON.stringify(k)}: return ${v};`).join("\n");
  const fnBody = `switch(cmd) {\n${switchCases}\ndefault: throw new Error("Unhandled Tauri mock command: " + cmd);\n}`;
  await mockTauriIPC(page, fnBody);
}
