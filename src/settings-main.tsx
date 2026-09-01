import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import SettingsScreen from "./components/settings/SettingsScreen";
import "./styles/index.css";
import "./styles/globals/reset.css";
import "./styles/globals/tokens.css";
import "./styles/globals/themes.css";

const root = document.getElementById("root");
if (!root) {
  throw new Error("settings_root_missing");
}

createRoot(root).render(
  <StrictMode>
    <SettingsScreen />
  </StrictMode>,
);
