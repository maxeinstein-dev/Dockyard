import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { WidgetCommandError, WidgetInstance } from "../../core/widgets";
import { WIDGET_FIXTURES } from "../../core/widgets.fixtures";
import type { WidgetUpdate } from "../contract";
import NetworkSettings from "./NetworkSettings";

type OnUpdate = (update: WidgetUpdate<"network">) => Promise<void>;

const NETWORK = WIDGET_FIXTURES["network-default"];

function renderSettings(
  instance: WidgetInstance<"network"> = NETWORK,
  {
    onUpdate = vi.fn<OnUpdate>(() => Promise.resolve()),
    onRemove = vi.fn(() => Promise.resolve()),
    onBack = vi.fn(),
  }: {
    onUpdate?: OnUpdate;
    onRemove?: () => Promise<void>;
    onBack?: () => void;
  } = {},
) {
  const view = render(
    <NetworkSettings
      instance={instance}
      onUpdate={onUpdate}
      onRemove={onRemove}
      onBack={onBack}
    />,
  );
  const rerender = (next: WidgetInstance<"network">) =>
    view.rerender(
      <NetworkSettings
        instance={next}
        onUpdate={onUpdate}
        onRemove={onRemove}
        onBack={onBack}
      />,
    );
  return { ...view, rerender, onUpdate, onRemove, onBack };
}

const rejected = (code: WidgetCommandError["code"]) => () =>
  Promise.reject({ code } satisfies WidgetCommandError);

const widths = () => screen.getByRole("group", { name: "Largura no dock" });
const width = (name: string) => within(widths()).getByRole("button", { name });
const pressed = () =>
  within(widths())
    .getAllByRole("button")
    .filter((button) => button.getAttribute("aria-pressed") === "true")
    .map((button) => button.textContent);
const labelField = () => screen.getByRole("textbox", { name: "Rótulo" });

describe("NetworkSettings", () => {
  it("is the Ajustes da rede view, focused on Voltar", () => {
    const { onBack } = renderSettings();

    expect(
      screen.getByRole("heading", { name: "Ajustes da rede" }),
    ).toBeInTheDocument();
    const back = screen.getByRole("button", { name: "Voltar" });
    expect(back).toHaveFocus();
    fireEvent.click(back);
    expect(onBack).toHaveBeenCalledTimes(1);
  });

  it("has only the label, the width and Remover do dock", () => {
    renderSettings();

    expect(labelField()).toHaveAttribute("placeholder", "Atividade de rede");
    expect(screen.getAllByRole("group")).toHaveLength(1);
    expect(
      within(widths())
        .getAllByRole("button")
        .map((button) => button.textContent),
    ).toEqual(["1 ícone", "2 ícones", "3 ícones"]);
    expect(pressed()).toEqual(["2 ícones"]);
    expect(
      screen.getByText(
        "Com 2 ícones, o dock mostra o gráfico do último minuto; com 3, maior.",
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Remover do dock" }),
    ).toBeEnabled();
    expect(screen.getByText("Salvo automaticamente")).toBeInTheDocument();
  });

  it("saves a width at once, as a number, with the label", async () => {
    const { onUpdate } = renderSettings({ ...NETWORK, label: "Wi-Fi" });

    await act(async () => fireEvent.click(width("3 ícones")));
    expect(onUpdate).toHaveBeenCalledExactlyOnceWith({
      label: "Wi-Fi",
      config: { width: 3 },
    });
    expect(pressed()).toEqual(["3 ícones"]);
  });

  it("does not save the width already chosen", async () => {
    const { onUpdate } = renderSettings();

    await act(async () => fireEvent.click(width("2 ícones")));
    expect(onUpdate).not.toHaveBeenCalled();
  });

  it("saves the label on blur, an empty one as none", async () => {
    const { onUpdate } = renderSettings({ ...NETWORK, label: "Wi-Fi" });

    fireEvent.change(labelField(), { target: { value: "   " } });
    await act(async () => fireEvent.blur(labelField()));
    expect(onUpdate).toHaveBeenCalledExactlyOnceWith({
      label: null,
      config: NETWORK.config,
    });
  });

  it("puts the width back and says why when the save is refused", async () => {
    renderSettings(NETWORK, { onUpdate: rejected("notFound") });

    await act(async () => fireEvent.click(width("1 ícone")));
    expect(pressed()).toEqual(["2 ícones"]);
    expect(screen.getByRole("alert")).toHaveTextContent(
      "Este widget não está mais no dock.",
    );
  });

  it("shows a new accepted instance", () => {
    const { rerender } = renderSettings();

    rerender({ ...NETWORK, label: "Cabo", config: { width: 1 } });
    expect(labelField()).toHaveValue("Cabo");
    expect(pressed()).toEqual(["1 ícone"]);
  });

  it("removes the instance, and enables Remover again on refusal", async () => {
    const onRemove = vi.fn(rejected("writeFailed"));
    renderSettings(NETWORK, { onRemove });

    const remove = screen.getByRole("button", { name: "Remover do dock" });
    await act(async () => fireEvent.click(remove));
    expect(onRemove).toHaveBeenCalledTimes(1);
    expect(remove).toBeEnabled();
    expect(screen.getByRole("alert")).toHaveTextContent(
      "Não foi possível salvar a mudança. Tente de novo.",
    );
  });
});
