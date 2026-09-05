import {
  invokeCheckForUpdate,
  invokeInstallUpdate,
} from "./desktop/commands";
import {
  parseUpdateCheckResult,
  type UpdateCheckResult,
} from "./contracts/updates";

export { parseUpdateCheckResult, type UpdateCheckResult } from "./contracts/updates";

export async function checkForUpdate(): Promise<UpdateCheckResult> {
  const value = await invokeCheckForUpdate();
  const result = parseUpdateCheckResult(value);
  if (!result) throw new Error("invalid_update_check");
  return result;
}

export async function installUpdate(): Promise<void> {
  await invokeInstallUpdate();
}
