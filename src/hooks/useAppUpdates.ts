import { useState } from "react";
import {
  checkForUpdate,
  installUpdate as installApplicationUpdate,
  type UpdateCheckResult,
} from "../lib/updates";
import { errorMessage } from "../components/settings/settings-model";

export type AppUpdateStatus =
  | "idle"
  | "checking"
  | "up-to-date"
  | "available"
  | "installing"
  | "error";

export interface UseAppUpdatesResult {
  status: AppUpdateStatus;
  currentVersion: string | null;
  availableVersion: string | null;
  error: string | null;
  checkForUpdates(): Promise<void>;
  installUpdate(): Promise<void>;
}

export default function useAppUpdates(): UseAppUpdatesResult {
  const [status, setStatus] = useState<AppUpdateStatus>("idle");
  const [result, setResult] = useState<UpdateCheckResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  const checkForUpdates = async () => {
    if (status === "checking" || status === "installing") return;
    setStatus("checking");
    setError(null);
    try {
      const nextResult = await checkForUpdate();
      setResult(nextResult);
      setStatus(nextResult.availableVersion ? "available" : "up-to-date");
    } catch (checkError) {
      setResult(null);
      setStatus("error");
      setError(errorMessage(checkError));
    }
  };

  const installUpdate = async () => {
    if (status !== "available") return;
    setStatus("installing");
    setError(null);
    try {
      await installApplicationUpdate();
    } catch (installError) {
      setStatus("error");
      setError(errorMessage(installError));
    }
  };

  return {
    status,
    currentVersion: result?.currentVersion ?? null,
    availableVersion: result?.availableVersion ?? null,
    error,
    checkForUpdates,
    installUpdate,
  };
}
