import styles from "../../styles/widget/surface.module.css";

export default function WidgetHeader() {
  return (
    <header className={styles.header}>
      <div className={styles.title}>
        <h1 className={styles.titleText}>Token Tracing</h1>
      </div>
    </header>
  );
}
