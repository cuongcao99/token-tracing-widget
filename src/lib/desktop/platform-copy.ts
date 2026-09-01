export const INVALID_SOURCE_ROOT_MESSAGE =
  "Invalid source root. Use an absolute Windows path or an approved WSL path.";

export const NATIVE_SETTINGS_COPY = {
  saveFailed: "Could not save settings.",
  refreshFailed: "Settings were not applied because collection could not refresh.",
  sourceRootOpenFailed: "Could not open the source folder.",
  sourceRootInvalid: "That folder cannot be used as a source.",
  sourceRootUnavailable: "The source folder is unavailable.",
  traceHookReadFailed: "Could not read agent tracing settings.",
  traceHookWriteFailed: "Could not update the agent tracing hook.",
  traceHookInvalid: "The agent hook configuration is invalid; no changes were made.",
  traceHookUnavailable: "Agent tracing is unavailable in this installation.",
  invalidSettings: "Settings returned an invalid value.",
  unavailable: "Settings are unavailable.",
  previewFailed: "Could not preview settings.",
  closeFailed: "Could not close Settings.",
} as const;
