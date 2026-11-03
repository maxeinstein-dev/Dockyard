import { useId, useRef, useState, type KeyboardEvent } from "react";

export interface Option<V extends string> {
  value: V;
  label: string;
}

type OptionGroupProps<V extends string> = {
  /** Visible name of the group, also its accessible name. */
  label: string;
  options: readonly Option<V>[];
  /** The option shown as chosen, or null for none (a value no option
   * matches). */
  selected: V | null;
  /** Called when an option other than the chosen one is chosen. */
  onSelect: (value: V) => void;
  /** A line under the options that says what the setting does; it also
   * describes the group. */
  hint?: string;
};

/**
 * A single-choice segmented control (DESIGN.md "Controle segmentado"): a
 * labelled group of toggle buttons, the chosen one `aria-pressed`. One Tab
 * stop per group (roving tabindex, starting at the chosen option); the
 * arrows, Home and End move the focus inside the group without choosing,
 * and Enter or Space chooses the focused option. Choosing applies at once,
 * so moving through the options never changes a setting by itself.
 */
function OptionGroup<V extends string>({
  label,
  options,
  selected,
  onSelect,
  hint,
}: OptionGroupProps<V>) {
  const labelId = useId();
  const hintId = useId();
  const buttons = useRef<(HTMLButtonElement | null)[]>([]);
  /** Option holding the focus inside the group, if any. */
  const [focused, setFocused] = useState<number | null>(null);

  const selectedIndex = options.findIndex(
    (option) => option.value === selected,
  );
  const tabStop = focused ?? (selectedIndex >= 0 ? selectedIndex : 0);

  function moveFocus(index: number) {
    setFocused(index);
    buttons.current[index]?.focus();
  }

  function onKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    const last = options.length - 1;
    const current = buttons.current.findIndex(
      (button) => button === document.activeElement,
    );
    const from = current >= 0 ? current : tabStop;
    let next: number;
    switch (event.key) {
      case "ArrowRight":
      case "ArrowDown":
        next = from === last ? 0 : from + 1;
        break;
      case "ArrowLeft":
      case "ArrowUp":
        next = from === 0 ? last : from - 1;
        break;
      case "Home":
        next = 0;
        break;
      case "End":
        next = last;
        break;
      default:
        return;
    }
    event.preventDefault();
    moveFocus(next);
  }

  return (
    <div className="option-row">
      <span id={labelId} className="option-label">
        {label}
      </span>
      <div
        className="segmented"
        role="group"
        aria-labelledby={labelId}
        aria-describedby={hint ? hintId : undefined}
        onKeyDown={onKeyDown}
        onBlur={(event) => {
          // Leaving the group: the next Tab into it lands on the chosen
          // option again.
          if (!event.currentTarget.contains(event.relatedTarget)) {
            setFocused(null);
          }
        }}
      >
        {options.map((option, index) => (
          <button
            key={option.value}
            ref={(button) => {
              buttons.current[index] = button;
            }}
            type="button"
            aria-pressed={index === selectedIndex}
            tabIndex={index === tabStop ? 0 : -1}
            onFocus={() => setFocused(index)}
            onClick={() => {
              setFocused(index);
              if (index !== selectedIndex) onSelect(option.value);
            }}
          >
            {option.label}
          </button>
        ))}
      </div>
      {hint ? (
        <span id={hintId} className="option-hint">
          {hint}
        </span>
      ) : null}
    </div>
  );
}

export default OptionGroup;
