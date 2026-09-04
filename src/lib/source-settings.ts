import {
  invokePickSourceRoot,
  invokeSourceSettings,
  invokeUpdateSourceSettings,
} from "./desktop/commands";
import {
  parseSourceSettings,
  type SourceSettings,
  type SourcePlatform,
  type SourceSettingsSnapshot,
} from "./contracts/source-settings";
import type { ProviderId } from "./provider";

export type { ProviderId } from "./provider";
export {
  parseSourceSettings,
  type SourceSettings,
  type SourcePlatform,
  type SourceSettingsSnapshot,
} from "./contracts/source-settings";

export async function getSourceSettings(): Promise<SourceSettingsSnapshot> {
  const value = await invokeSourceSettings();
  const settings = parseSourceSettings(value);
  if (!settings) {
    throw new Error("invalid_source_settings");
  }
  return settings;
}

export async function pickSourceRoot(
  provider: ProviderId,
  platform: SourcePlatform,
): Promise<SourceSettingsSnapshot | null> {
  const value = await invokePickSourceRoot(provider, platform);
  if (value === null) return null;

  const settings = parseSourceSettings(value);
  if (!settings) {
    throw new Error("invalid_source_settings");
  }
  return settings;
}

export async function updateSourceSettings(
  settings: SourceSettings,
): Promise<SourceSettingsSnapshot> {
  const value = await invokeUpdateSourceSettings(settings);
  const nextSettings = parseSourceSettings(value);
  if (!nextSettings) {
    throw new Error("invalid_source_settings");
  }
  return nextSettings;
}
