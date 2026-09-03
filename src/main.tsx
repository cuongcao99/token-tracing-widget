import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import TokenTracingWidget from "./components/widget/TokenTracingWidget";
import "./styles/globals/reset.css";
import "./styles/globals/tokens.css";
import "./styles/globals/themes.css";
import "./styles/widget/surface.module.css";
import "./styles/widget/provider.module.css";
import "./styles/widget/metrics.module.css";
import "./styles/widget/total.module.css";
import "./styles/widget/loading.module.css";
import "./styles/shared/branding.module.css";
import "./styles/shared/window-controls.module.css";

const root = document.getElementById("root");

if (!root) {
  throw new Error("The application root element is missing");
}

createRoot(root).render(
  <StrictMode>
    <TokenTracingWidget />
  </StrictMode>,
);
