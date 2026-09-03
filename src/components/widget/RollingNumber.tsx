import { useLayoutEffect, useRef, useState, type CSSProperties } from "react";
import styles from "../../styles/widget/rolling-number.module.css";

export interface RollingNumberProps {
  value: string;
}

interface DigitTransition {
  id: number;
  from: number;
  to: number;
  steps: number;
}

type RollingStyle = CSSProperties & {
  "--rolling-delay": string;
  "--rolling-distance": string;
};

function isDigit(character: string): boolean {
  return character >= "0" && character <= "9";
}

function forwardSteps(from: number, to: number): number {
  return (to - from + 10) % 10;
}

function RollingDigit({ digit, position }: { digit: number; position: number }) {
  const previousDigitRef = useRef(digit);
  const transitionIdRef = useRef(0);
  const [transition, setTransition] = useState<DigitTransition | null>(null);

  useLayoutEffect(() => {
    const previous = previousDigitRef.current;
    if (previous === digit) return;

    previousDigitRef.current = digit;
    transitionIdRef.current += 1;
    setTransition({
      id: transitionIdRef.current,
      from: previous,
      to: digit,
      steps: forwardSteps(previous, digit),
    });
  }, [digit]);

  const handleAnimationEnd = () => {
    const completedId = transition?.id;
    if (completedId === undefined) return;
    setTransition((current) => (current?.id === completedId ? null : current));
  };

  const faces = transition
    ? Array.from({ length: transition.steps + 1 }, (_, index) =>
        (transition.from + index) % 10,
      )
    : [digit];
  const rollingStyle: RollingStyle | undefined = transition
    ? {
        "--rolling-delay": `${position * 24}ms`,
        "--rolling-distance": `-${transition.steps}em`,
      }
    : undefined;

  return (
    <span
      className={styles.window}
      data-from={transition?.from}
      data-position={position}
      data-rolling={transition ? "true" : "false"}
      data-to={transition?.to}
    >
      <span
        key={transition?.id ?? "static"}
        className={`${styles.track} ${transition ? styles.rolling : ""}`}
        onAnimationEnd={transition ? handleAnimationEnd : undefined}
        style={rollingStyle}
      >
        {faces.map((face, index) => (
          <span className={styles.face} key={`${face}-${index}`}>
            {face}
          </span>
        ))}
      </span>
    </span>
  );
}

export default function RollingNumber({ value }: RollingNumberProps) {
  const digitPositions = new Map<number, number>();
  let position = 0;
  for (let index = value.length - 1; index >= 0; index -= 1) {
    if (isDigit(value[index])) {
      digitPositions.set(index, position);
      position += 1;
    }
  }

  return (
    <span
      className={styles.number}
      data-testid="rolling-number"
      data-value={value}
      aria-hidden="true"
    >
      {Array.from(value).map((character, index) => {
        const digitPosition = digitPositions.get(index);
        return digitPosition === undefined ? (
          <span className={styles.separator} key={`separator-${index}`}>
            {character}
          </span>
        ) : (
          <RollingDigit
            digit={Number(character)}
            key={`digit-${digitPosition}`}
            position={digitPosition}
          />
        );
      })}
    </span>
  );
}
