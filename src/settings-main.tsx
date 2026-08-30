import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import Settings from "./Settings";
import "./styles.css";

const root = document.getElementById("root");
if (!root) {
  throw new Error("settings_root_missing");
}

createRoot(root).render(
  <StrictMode>
    <Settings />
  </StrictMode>,
);
