import {
  invokeTraceHookStatus,
  invokeUpdateTraceHook,
} from "./desktop/commands";
import {
  parseTraceHooks,
  type TraceHookStatus,
  type TraceHooksSnapshot,
} from "./contracts/trace-hooks";
import type { ProviderId } from "./provider";

export {
  parseTraceHooks,
  type TraceHookStatus,
  type TraceHooksSnapshot,
} from "./contracts/trace-hooks";

export interface TraceHookSettings {
  provider: ProviderId;
  enabled: boolean;
}

export async function getTraceHookStatus(): Promise<TraceHooksSnapshot> {
  const value = await invokeTraceHookStatus();
  const snapshot = parseTraceHooks(value);
  if (!snapshot) throw new Error("invalid_trace_hook_status");
  return snapshot;
}

export async function updateTraceHook(
  settings: TraceHookSettings,
): Promise<TraceHooksSnapshot> {
  const value = await invokeUpdateTraceHook(settings);
  const snapshot = parseTraceHooks(value);
  if (!snapshot) throw new Error("invalid_trace_hook_status");
  return snapshot;
}
