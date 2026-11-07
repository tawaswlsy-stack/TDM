/** The theme the settings name: light, dark, or the system's. */
export const PALETTES = ["tdm", "ocean", "forest", "sunset"] as const;
export type Palette = (typeof PALETTES)[number];

const PALETTE_KEY = "tdm-color-palette";

function isPalette(value: string | null): value is Palette {
  return PALETTES.some((palette) => palette === value);
}

export function getPalette(): Palette {
  const saved = localStorage.getItem(PALETTE_KEY);
  return isPalette(saved) ? saved : "tdm";
}

export function setPalette(palette: Palette): void {
  localStorage.setItem(PALETTE_KEY, palette);
  document.documentElement.dataset.palette = palette;
}

export function setTheme(theme: string) {
  if (theme === "light" || theme === "dark") document.documentElement.dataset.theme = theme;
  else delete document.documentElement.dataset.theme;
  document.documentElement.dataset.palette = getPalette();
}

if (typeof window !== "undefined") {
  window.addEventListener("storage", (event) => {
    if (event.key === PALETTE_KEY) document.documentElement.dataset.palette = getPalette();
  });
}
