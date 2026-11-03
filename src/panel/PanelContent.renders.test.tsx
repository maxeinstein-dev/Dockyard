import { render } from "@testing-library/react";
import type { ComponentProps } from "react";
import { describe, expect, it, vi } from "vitest";
import type { OpenNow } from "../core/openApps";
import { WIDGET_FIXTURES } from "../core/widgets.fixtures";
import PanelContent from "./PanelContent";
import { MAIN_VIEW } from "./view";

/* M8: the panel's root follows the open programs, the pinned ones and the
 * workspaces; a widget's view shows none of them. When one of those
 * changes, the open view of a note is not drawn again. */

const { drawn } = vi.hoisted(() => ({ drawn: vi.fn() }));
vi.mock("../widgets/notes/NotePanel", () => ({
  default: () => {
    drawn();
    return <p>nota</p>;
  },
}));

const NOTE = WIDGET_FIXTURES["note-default"];
const open = (count: number): OpenNow => ({
  apps: new Map(
    Array.from({ length: count }, (_, n) => [`app${n}`, { count: 1 } as never]),
  ),
  others: [],
  loaded: true,
});

function content(props: Partial<ComponentProps<typeof PanelContent>> = {}) {
  return (
    <PanelContent
      panelId={{ widget: NOTE.id }}
      settings={null}
      widgets={null}
      apps={null}
      app={null}
      open={open(0)}
      other={null}
      workspaces={null}
      instance={NOTE}
      view={MAIN_VIEW}
      onView={vi.fn()}
      onRemoved={vi.fn()}
      {...props}
    />
  );
}

describe("a widget's view", () => {
  it("is not drawn again when only the open programs change", () => {
    const { rerender } = render(content());
    expect(drawn).toHaveBeenCalledTimes(1);

    rerender(content({ open: open(1), onView: vi.fn() }));
    rerender(content({ open: open(2), onView: vi.fn() }));

    expect(drawn).toHaveBeenCalledTimes(1);
  });

  it("is drawn again when its widget changes", () => {
    drawn.mockClear();
    const { rerender } = render(content());
    rerender(content({ instance: { ...NOTE, label: "Outra" } }));
    expect(drawn).toHaveBeenCalledTimes(2);
  });
});
