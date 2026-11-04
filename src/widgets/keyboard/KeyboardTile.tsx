import "../system/system.css";

/** The tile of a Layout do teclado outside the dock: a keyboard. The dock
 * draws its own natively, with the layout in front (`PTB`). Decorative. */
function KeyboardTile() {
  return (
    <span className="sys-tile" aria-hidden="true">
      <svg viewBox="0 0 24 24">
        <path d="M4.5 6.5h15a1.5 1.5 0 0 1 1.5 1.5v8a1.5 1.5 0 0 1 -1.5 1.5h-15a1.5 1.5 0 0 1 -1.5 -1.5v-8a1.5 1.5 0 0 1 1.5 -1.5M7 10h.01M10 10h.01M13 10h.01M16 10h.01M8 14h8" />
      </svg>
    </span>
  );
}

export default KeyboardTile;
