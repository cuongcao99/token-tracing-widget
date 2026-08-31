import { formatTokens } from "./widget-types";

interface WidgetTotalProps {
  tokens: number;
}

export default function WidgetTotal({ tokens }: WidgetTotalProps) {
  return (
    <footer className="widget-total">
      <span>Total</span>
      <strong aria-label={`Total: ${formatTokens(tokens)} tokens`}>
        {formatTokens(tokens)}
      </strong>
    </footer>
  );
}
