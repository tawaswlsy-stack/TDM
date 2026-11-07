import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { getPalette, PALETTES, setPalette, setTheme } from "../src/lib/theme";

describe("appearance palettes", () => {
  const saved = new Map<string, string>();
  const dataset: Record<string, string> = {};

  beforeEach(() => {
    saved.clear();
    for (const key of Object.keys(dataset)) delete dataset[key];
    vi.stubGlobal("localStorage", {
      getItem: (key: string) => saved.get(key) ?? null,
      setItem: (key: string, value: string) => saved.set(key, value),
    });
    vi.stubGlobal("document", { documentElement: { dataset } });
  });

  afterEach(() => vi.unstubAllGlobals());

  it("offers the four built-in color templates and defaults to TDM", () => {
    expect(PALETTES).toEqual(["tdm", "ocean", "forest", "sunset"]);
    expect(getPalette()).toBe("tdm");
  });

  it("uses only a saved, supported template", () => {
    saved.set("tdm-color-palette", "forest");
    expect(getPalette()).toBe("forest");
    saved.set("tdm-color-palette", "unknown");
    expect(getPalette()).toBe("tdm");
  });

  it("persists the selected template and reapplies it with each appearance mode", () => {
    setPalette("ocean");
    setTheme("dark");
    expect(saved.get("tdm-color-palette")).toBe("ocean");
    expect(dataset).toEqual({ palette: "ocean", theme: "dark" });
    setTheme("system");
    expect(dataset).toEqual({ palette: "ocean" });
  });
});
