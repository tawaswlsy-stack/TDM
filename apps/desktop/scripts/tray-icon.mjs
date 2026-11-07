// The macOS tray template uses the supplied monochrome TDM glyph in black.
//
//   node scripts/tray-icon.mjs      (writes src-tauri/icons/tray-template.png)
import { readFileSync, writeFileSync } from "node:fs";
import { Resvg } from "@resvg/resvg-js";

const svg = readFileSync(new URL("../../../assets/icon/TDM_Glyph_White.svg", import.meta.url), "utf8");
const png = new Resvg(svg.replaceAll("#FFFFFF", "#000000"), {
  fitTo: { mode: "width", value: 44 },
  background: "rgba(0, 0, 0, 0)",
})
  .render()
  .asPng();
writeFileSync(new URL("../src-tauri/icons/tray-template.png", import.meta.url), png);
console.log(`src-tauri/icons/tray-template.png: ${png.length} bytes`);
