import "../system/system.css";

/** The tile of a Relógio mundial outside the dock: a globe. The dock draws
 * the live one natively, a line per place (`native_dock/faces.rs`,
 * `world_clock`). Decorative. */
function WorldClockTile() {
  return (
    <span className="sys-tile" aria-hidden="true">
      <svg viewBox="0 0 24 24">
        <path d="M3.5 12a8.5 8.5 0 1 0 17 0a8.5 8.5 0 1 0 -17 0M3.5 12h17M12 3.5c2.5 2.5 3.5 5.5 3.5 8.5s-1 6 -3.5 8.5M12 3.5c-2.5 2.5 -3.5 5.5 -3.5 8.5s1 6 3.5 8.5" />
      </svg>
    </span>
  );
}

export default WorldClockTile;
