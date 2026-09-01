import { formatTokens } from "./widget-types";
import styles from "../../styles/widget/total.module.css";

interface WidgetTotalProps {
  tokens: number;
}

export default function WidgetTotal({ tokens }: WidgetTotalProps) {
  return (
    <footer className={styles.root}>
      <span className={styles.label}>Total</span>
      <strong className={styles.value} aria-label={`Total: ${formatTokens(tokens)} tokens`}>
        {formatTokens(tokens)}
      </strong>
    </footer>
  );
}
