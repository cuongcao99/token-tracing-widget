export default function WindowGrip() {
  return (
    <span className="window-grip" data-testid="window-grip" aria-hidden="true">
      {Array.from({ length: 6 }, (_, index) => (
        <span className="window-grip__dot" data-testid="window-grip-dot" key={index} />
      ))}
    </span>
  );
}
