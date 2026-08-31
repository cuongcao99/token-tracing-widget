import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import TokenTracingWidget from "./components/widget/TokenTracingWidget";
import "./styles/index.css";

const root = document.getElementById("root");

if (!root) {
  throw new Error("The application root element is missing");
}

createRoot(root).render(
  <StrictMode>
    <TokenTracingWidget />
  </StrictMode>,
);
