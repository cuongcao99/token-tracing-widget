import type { UsageState } from "../../lib/usage-summary";
import WindowGrip from "../shared/WindowGrip";
import { headerStateLabel } from "./widget-types";

interface WidgetHeaderProps {
  state: UsageState;
}

export default function WidgetHeader({ state }: WidgetHeaderProps) {
  return (
    <header className="widget-header" data-tauri-drag-region="">
      <div className="widget-header__title">
        <WindowGrip />
        <h1>Token Tracing</h1>
      </div>
      <div
        className={`widget-status widget-status--${state}`}
        role="status"
        aria-live="polite"
        aria-atomic="true"
      >
        <span className="widget-status__dot" aria-hidden="true" />
        <span>{headerStateLabel(state)}</span>
      </div>
    </header>
  );
}
