import {
  DesktopError,
  createDesktopContract,
  type DesktopAdapter,
  type DesktopContract,
} from "./contract";
import type { DesktopCommands, DesktopEvents } from "../generated/desktop-contract";

type CommandHandlers = {
  [K in keyof DesktopCommands]?: (
    args: DesktopCommands[K]["args"],
  ) => DesktopCommands[K]["result"] | Promise<DesktopCommands[K]["result"]>;
};

/** Strict in-memory Adapter for browser and unit-test stand-ins. */
export class BrowserMockAdapter implements DesktopAdapter {
  readonly contract: DesktopContract;
  readonly calls: Array<{ command: keyof DesktopCommands; args: unknown }> = [];

  private readonly listeners = new Map<string, Set<(payload: unknown) => void>>();

  constructor(private readonly handlers: CommandHandlers = {}) {
    this.contract = createDesktopContract(this);
  }

  async callRaw(command: string, args: unknown): Promise<unknown> {
    const typedCommand = command as keyof DesktopCommands;
    this.calls.push({ command: typedCommand, args });
    const handler = this.handlers[typedCommand] as ((args: unknown) => unknown) | undefined;
    if (!handler) {
      throw new DesktopError(
        "contract",
        command,
        "unhandled_mock_command",
        `Browser mock has no handler for desktop command: ${command}`,
      );
    }
    return handler(args);
  }

  async subscribeRaw(event: string, receive: (payload: unknown) => void): Promise<() => void> {
    const listeners = this.listeners.get(event) ?? new Set<(payload: unknown) => void>();
    listeners.add(receive);
    this.listeners.set(event, listeners);
    return () => listeners.delete(receive);
  }

  emit<K extends keyof DesktopEvents>(event: K, payload: DesktopEvents[K]): void {
    for (const listener of this.listeners.get(event) ?? []) listener(payload);
  }
}
