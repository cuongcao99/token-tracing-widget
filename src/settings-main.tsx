import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import SettingsScreen from "./components/settings/SettingsScreen";
import "./styles/index.css";

const root = document.getElementById("root");
if (!root) {
  throw new Error("settings_root_missing");
}

createRoot(root).render(
  <StrictMode>
    <SettingsScreen />
  </StrictMode>,
);
