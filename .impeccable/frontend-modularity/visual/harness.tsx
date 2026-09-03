/*
 * Ignored visual-review harness.
 *
 * It mounts the production React entry point while providing only synthetic
 * values through the Tauri IPC boundary. No provider records, source files,
 * credentials, or local settings are read by this page.
 */

const FIXED_NOW = Date.parse("2026-09-01T12:00:00+07:00");
const mode = new URLSearchParams(window.location.search).get("mode") === "light"
  ? "light"
  : "dark";
const surface = new URLSearchParams(window.location.search).get("surface") === "settings"
  ? "settings"
  : "widget";
const providerCount = Math.max(
  0,
  Math.min(
    2,
    Number(new URLSearchParams(window.location.search).get("providers") ?? "2"),
  ),
);

Date.now = () => FIXED_NOW;
document.documentElement.dataset.visualSurface = surface;
document.documentElement.dataset.visualMode = mode;

const currentWidgetSettings = {
  darkMode: mode === "dark",
  theme: "claude",
  visibleProviders: [
    { provider: "claude", visible: providerCount >= 1 },
    { provider: "codex", visible: providerCount >= 2 },
  ],
};

const currentSourceSettings = {
  sources: [
    {
      provider: "claude",
      enabled: true,
      rootOverride: "C:\\Fixture\\Claude\\projects",
    },
    {
      provider: "codex",
      enabled: true,
      rootOverride: "C:\\Fixture\\Codex\\sessions",
    },
  ],
};

const currentUsageSummary = {
  state: "active",
  todayTokens: 50880,
  sourceHealth: [
    { provider: "claude", state: "detected" },
    { provider: "codex", state: "limited" },
  ],
  providers: [
    {
      provider: "claude",
      state: "active",
      currentSessionTokens: 12480,
      todayTokens: 38240,
      lastUpdatedAt: "2026-09-01T11:57:00+07:00",
    },
    {
      provider: "codex",
      state: "idle",
      currentSessionTokens: 6320,
      todayTokens: 12640,
      lastUpdatedAt: "2026-09-01T11:48:00+07:00",
    },
  ],
};

type Callback = (event: { event: string; id: number; payload: unknown }) => void;
const callbacks = new Map<number, Callback>();
const listeners = new Map<string, Set<number>>();
let nextCallbackId = 1;
let nextEventId = 1;

function registerCallback(callback: Callback): number {
  const id = nextCallbackId++;
  callbacks.set(id, callback);
  return id;
}

function registerListener(event: string, callbackId: number): void {
  const eventListeners = listeners.get(event) ?? new Set<number>();
  eventListeners.add(callbackId);
  listeners.set(event, eventListeners);
}

function unregisterListener(event: string, callbackId: number): void {
  listeners.get(event)?.delete(callbackId);
  callbacks.delete(callbackId);
}

function emitFixture(event: string, payload: unknown): void {
  for (const callbackId of listeners.get(event) ?? []) {
    callbacks.get(callbackId)?.({ event, id: nextEventId++, payload });
  }
}

function clone<T>(value: T): T {
  return structuredClone(value);
}

const tauriInternals = {
  metadata: { currentWindow: { label: surface } },
  transformCallback: registerCallback,
  unregisterCallback: (callbackId: number) => callbacks.delete(callbackId),
  invoke: async (command: string, args: Record<string, unknown> = {}) => {
    switch (command) {
      case "get_usage_summary":
        return clone(currentUsageSummary);
      case "get_widget_settings":
        return clone(currentWidgetSettings);
      case "get_source_settings":
        return clone(currentSourceSettings);
      case "update_widget_settings":
        return clone((args.settings as typeof currentWidgetSettings) ?? currentWidgetSettings);
      case "update_source_settings":
        return clone(currentSourceSettings);
      case "pick_source_root":
        return null;
      case "plugin:event|listen": {
        const event = String(args.event ?? "");
        registerListener(event, Number(args.handler));
        return nextEventId++;
      }
      case "plugin:event|unlisten":
        unregisterListener(String(args.event ?? ""), Number(args.eventId));
        return null;
      case "plugin:event|emit":
        emitFixture(String(args.event ?? ""), args.payload);
        return null;
      case "plugin:window|inner_size":
        return { type: "Physical", width: window.innerWidth, height: window.innerHeight };
      case "plugin:window|scale_factor":
        return 1;
      case "plugin:window|set_size_constraints":
      case "plugin:window|set_size":
      case "plugin:window|start_dragging":
      case "plugin:window|start_resize_dragging":
      case "plugin:window|close":
        return null;
      default:
        throw new Error(`visual_harness_unknown_command:${command}`);
    }
  },
};

(window as unknown as { __TAURI_INTERNALS__: typeof tauriInternals }).__TAURI_INTERNALS__ = tauriInternals;
(window as unknown as { __TAURI_EVENT_PLUGIN_INTERNALS__: { unregisterListener: typeof unregisterListener } }).__TAURI_EVENT_PLUGIN_INTERNALS__ = {
  unregisterListener,
};

if (surface === "widget") {
  await import("/src/main.tsx");
} else {
  await import("/src/settings-main.tsx");
}
