import "../system/system.css";

/** The tile of a Capturas de tela outside the dock: a camera, as the dock
 * draws it natively (`native_dock/faces.rs`). Decorative. */
function ScreenshotsTile() {
  return (
    <span className="sys-tile" aria-hidden="true">
      <svg viewBox="0 0 24 24">
        <path d="M4.5 8.5a2 2 0 0 1 2 -2h2l1.5 -2h4l1.5 2h2a2 2 0 0 1 2 2v9a2 2 0 0 1 -2 2h-11a2 2 0 0 1 -2 -2zM9 13a3 3 0 1 0 6 0a3 3 0 1 0 -6 0" />
      </svg>
    </span>
  );
}

export default ScreenshotsTile;
