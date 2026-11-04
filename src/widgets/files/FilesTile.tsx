import "../system/system.css";

/** The tile of an Arquivos outside the dock: a document with a clock's
 * hand, as the dock draws it natively (`native_dock/faces.rs`).
 * Decorative. */
function FilesTile() {
  return (
    <span className="sys-tile" aria-hidden="true">
      <svg viewBox="0 0 24 24">
        <path d="M7 3.5h6.5l4 4v13h-10.5a1.5 1.5 0 0 1 -1.5 -1.5v-14a1.5 1.5 0 0 1 1.5 -1.5zM13.5 3.5v4h4M12 11.5v3l2 1.5" />
      </svg>
    </span>
  );
}

export default FilesTile;
