import { useId, useState, type ReactNode } from "react";
import { opened } from "./advanced";
import "./parts.css";

/**
 * A group of rows of the Configurações app (spec 0014 R1): a small title
 * and, under it, a card with the rows. In the panel (`grouped` off) the rows
 * stay as they were, one after the other.
 */
function SettingsGroup({
  title,
  grouped,
  children,
}: {
  title: string;
  grouped: boolean;
  children: ReactNode;
}) {
  const titleId = useId();
  if (!grouped) return <>{children}</>;
  return (
    <section className="settings-group" aria-labelledby={titleId}>
      <h2 id={titleId} className="settings-group-title">
        {title}
      </h2>
      <div className="settings-group-card">{children}</div>
    </section>
  );
}

/**
 * Avançado (spec 0029 §4): the last group of a section, closed at first.
 * Its title is a button that opens and closes it, and the line under it
 * says what it holds. Closed, the rows stay in the page but hidden, so the
 * search finds them and opens the group (`openAdvanced`).
 */
export function AdvancedGroup({
  id,
  title,
  summary,
  children,
}: {
  /** Which Avançado: the section it closes. */
  id: string;
  title: string;
  summary: string;
  children: ReactNode;
}) {
  const titleId = useId();
  const summaryId = useId();
  const cardId = useId();
  const [open, setOpen] = useState(() => opened.has(id));
  const toggle = () => {
    const next = !open;
    if (next) opened.add(id);
    else opened.delete(id);
    setOpen(next);
  };
  return (
    <section
      className="settings-group settings-advanced"
      aria-labelledby={titleId}
      data-open={open ? "" : undefined}
    >
      <h2 id={titleId} className="settings-group-title">
        <button
          type="button"
          className="settings-advanced-toggle"
          aria-expanded={open}
          aria-controls={cardId}
          aria-describedby={summaryId}
          onClick={toggle}
        >
          {title}
        </button>
      </h2>
      <p id={summaryId} className="settings-advanced-summary">
        {summary}
      </p>
      <div id={cardId} className="settings-group-card" hidden={!open}>
        {children}
      </div>
    </section>
  );
}

export default SettingsGroup;
