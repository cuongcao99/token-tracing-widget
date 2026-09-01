import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import SettingsScreen from "./components/settings/SettingsScreen";
import "./styles/globals/reset.css";
import "./styles/globals/tokens.css";
import "./styles/globals/themes.css";
import "./styles/settings/surface.module.css";
import "./styles/settings/forms.module.css";
import "./styles/settings/theme-picker.module.css";
import "./styles/shared/branding.module.css";
import "./styles/shared/window-controls.module.css";

const root = document.getElementById("root");
if (!root) {
  throw new Error("settings_root_missing");
}

createRoot(root).render(
  <StrictMode>
    <SettingsScreen />
  </StrictMode>,
);
