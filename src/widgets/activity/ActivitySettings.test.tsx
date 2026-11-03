import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { WidgetCommandError, WidgetInstance } from "../../core/widgets";
import { WIDGET_FIXTURES } from "../../core/widgets.fixtures";
import type { WidgetUpdate } from "../contract";
import ActivitySettings from "./ActivitySettings";

type OnUpdate = (update: WidgetUpdate<"activity">) => Promise<void>;

const ACTIVITY = WIDGET_FIXTURES["activity-default"];

function renderSettings(
  instance: WidgetInstance<"activity"> = ACTIVITY,
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
    <ActivitySettings
      instance={instance}
      onUpdate={onUpdate}
      onRemove={onRemove}
      onBack={onBack}
    />,
  );
  return { ...view, onUpdate, onRemove, onBack };
}

const rejected = (code: WidgetCommandError["code"]) => () =>
  Promise.reject({ code } satisfies WidgetCommandError);

const group = (name: string) => screen.getByRole("group", { name });
const option = (groupName: string, name: string) =>
  within(group(groupName)).getByRole("button", { name });
const pressed = (groupName: string) =>
  within(group(groupName))
    .getAllByRole("button")
    .filter((button) => button.getAttribute("aria-pressed") === "true")
    .map((button) => button.textContent);
const labelField = () => screen.getByRole("textbox", { name: "Rótulo" });

describe("ActivitySettings", () => {
  it("is the Ajustes da atividade view, focused on Voltar", () => {
    const { onBack } = renderSettings();

    expect(
      screen.getByRole("heading", { name: "Ajustes da atividade" }),
    ).toBeInTheDocument();
    const back = screen.getByRole("button", { name: "Voltar" });
    expect(back).toHaveFocus();
    fireEvent.click(back);
    expect(onBack).toHaveBeenCalledTimes(1);
  });

  it("shows the instance's choices, with every option of each", () => {
    renderSettings(WIDGET_FIXTURES["activity-sample"]);

    expect(labelField()).toHaveValue("");
    expect(labelField()).toHaveAttribute("placeholder", "Atividade");
    expect(
      within(group("Mostrar"))
        .getAllByRole("button")
        .map((button) => button.textContent),
    ).toEqual(["Os dois", "Processador", "Memória"]);
    expect(
      within(group("Estilo no dock"))
        .getAllByRole("button")
        .map((button) => button.textContent),
    ).toEqual(["Barras", "Anéis", "Números"]);
    expect(
      within(group("Largura no dock"))
        .getAllByRole("button")
        .map((button) => button.textContent),
    ).toEqual(["1 ícone", "2 ícones", "3 ícones"]);

    expect(pressed("Mostrar")).toEqual(["Processador"]);
    expect(pressed("Estilo no dock")).toEqual(["Anéis"]);
    expect(pressed("Largura no dock")).toEqual(["3 ícones"]);
    expect(
      screen.getByText(
        "Com 3 ícones, o dock mostra também o processador no último minuto.",
      ),
    ).toBeInTheDocument();
    expect(screen.getByText("Salvo automaticamente")).toBeInTheDocument();
  });

  it.each([
    ["Mostrar", "Memória", { show: "memory" }],
    ["Estilo no dock", "Números", { style: "numbers" }],
    ["Largura no dock", "2 ícones", { width: 2 }],
  ] as const)(
    "saves %s at once, label and config in full",
    async (groupName, name, change) => {
      const { onUpdate } = renderSettings();

      await act(async () => fireEvent.click(option(groupName, name)));
      expect(onUpdate).toHaveBeenCalledExactlyOnceWith({
        label: null,
        config: { ...ACTIVITY.config, ...change },
      });
      expect(pressed(groupName)).toEqual([name]);
    },
  );

  it("saves the label on Enter", async () => {
    const { onUpdate } = renderSettings();

    fireEvent.change(labelField(), { target: { value: "  Notebook " } });
    await act(async () => {
      fireEvent.keyDown(labelField(), { key: "Enter" });
    });
    expect(onUpdate).toHaveBeenCalledExactlyOnceWith({
      label: "Notebook",
      config: ACTIVITY.config,
    });
  });

  it("refuses a label over 40 characters without saving", async () => {
    const { onUpdate } = renderSettings();

    fireEvent.change(labelField(), { target: { value: "x".repeat(41) } });
    await act(async () => fireEvent.blur(labelField()));
    expect(onUpdate).not.toHaveBeenCalled();
    expect(screen.getByRole("alert")).toHaveTextContent(
      "Use de 1 a 40 caracteres, com ao menos um visível.",
    );
    expect(labelField()).toHaveAttribute("aria-invalid", "true");
    expect(labelField()).toHaveValue("");
  });

  it("puts the choice back and says why when the save is refused", async () => {
    renderSettings(ACTIVITY, { onUpdate: rejected("invalidWidth") });

    await act(async () =>
      fireEvent.click(option("Largura no dock", "3 ícones")),
    );
    expect(pressed("Largura no dock")).toEqual(["1 ícone"]);
    expect(screen.getByRole("alert")).toHaveTextContent(
      "Essa largura não vale para este widget.",
    );
  });

  it("reports any other failure as a failed write", async () => {
    renderSettings(ACTIVITY, {
      onUpdate: () => Promise.reject(new Error("ipc")),
    });

    await act(async () => fireEvent.click(option("Mostrar", "Processador")));
    expect(screen.getByRole("alert")).toHaveTextContent(
      "Não foi possível salvar a mudança. Tente de novo.",
    );
  });

  it("removes the instance, disabled while it runs", async () => {
    let finish!: () => void;
    const onRemove = vi.fn(
      () =>
        new Promise<void>((resolve) => {
          finish = resolve;
        }),
    );
    renderSettings(ACTIVITY, { onRemove });

    const remove = screen.getByRole("button", { name: "Remover do dock" });
    await act(async () => fireEvent.click(remove));
    expect(onRemove).toHaveBeenCalledTimes(1);
    expect(remove).toBeDisabled();
    await act(async () => finish());
  });

  it("enables Remover again and says why when the removal is refused", async () => {
    renderSettings(ACTIVITY, { onRemove: rejected("notFound") });

    const remove = screen.getByRole("button", { name: "Remover do dock" });
    await act(async () => fireEvent.click(remove));
    expect(remove).toBeEnabled();
    expect(screen.getByRole("alert")).toHaveTextContent(
      "Este widget não está mais no dock.",
    );
  });
});
