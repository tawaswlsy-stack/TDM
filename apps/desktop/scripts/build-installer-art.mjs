import { mkdir, readFile, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { PNG } from "pngjs";
import { Resvg } from "@resvg/resvg-js";

const root = resolve(import.meta.dirname, "..");
const output = resolve(root, "src-tauri/installer");
const glyphBytes = await readFile(resolve(root, "../../assets/icon/TDM_Glyph_Cyan.svg"));
const glyph = `data:image/svg+xml;base64,${glyphBytes.toString("base64")}`;
const navy = "#07152b";
const cyan = "#14b8cf";

function artwork(width, height, body) {
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}">
    <defs>
      <linearGradient id="bg" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0" stop-color="${navy}"/><stop offset="1" stop-color="#104c70"/>
      </linearGradient>
    </defs>
    <rect width="100%" height="100%" fill="url(#bg)"/>
    ${body}
  </svg>`;
}

function brandIcon(x, y, size, radius = 0) {
  return `<image href="${glyph}" x="${x}" y="${y}" width="${size}" height="${size}" preserveAspectRatio="xMidYMid meet"${radius ? ` clip-path="inset(0 round ${radius}px)"` : ""}/>`;
}

function bitmap24(png) {
  const stride = Math.ceil((png.width * 3) / 4) * 4;
  const pixelBytes = stride * png.height;
  const file = Buffer.alloc(54 + pixelBytes);
  file.write("BM", 0);
  file.writeUInt32LE(file.length, 2);
  file.writeUInt32LE(54, 10);
  file.writeUInt32LE(40, 14);
  file.writeInt32LE(png.width, 18);
  file.writeInt32LE(png.height, 22);
  file.writeUInt16LE(1, 26);
  file.writeUInt16LE(24, 28);
  file.writeUInt32LE(pixelBytes, 34);
  for (let y = 0; y < png.height; y++) {
    const sourceY = png.height - 1 - y;
    for (let x = 0; x < png.width; x++) {
      const source = (sourceY * png.width + x) * 4;
      const target = 54 + y * stride + x * 3;
      const alpha = png.data[source + 3] / 255;
      file[target] = Math.round(png.data[source + 2] * alpha + 7 * (1 - alpha));
      file[target + 1] = Math.round(png.data[source + 1] * alpha + 21 * (1 - alpha));
      file[target + 2] = Math.round(png.data[source] * alpha + 43 * (1 - alpha));
    }
  }
  return file;
}

async function save(name, width, height, body) {
  const svg = artwork(width, height, body);
  const rendered = new Resvg(svg, { fitTo: { mode: "width", value: width } }).render().asPng();
  const png = PNG.sync.read(rendered);
  if (png.width !== width || png.height !== height) throw new Error(`${name}: unexpected image dimensions`);
  await writeFile(resolve(output, name), bitmap24(png));
}

await mkdir(output, { recursive: true });
await Promise.all([
  save(
    "nsis-header.bmp",
    150,
    57,
    `${brandIcon(7, 7, 43)}<text x="57" y="27" fill="#ffffff" font-family="Segoe UI, sans-serif" font-size="13" font-weight="700">TDM</text><text x="57" y="43" fill="${cyan}" font-family="Segoe UI, sans-serif" font-size="7" letter-spacing="1.2">TAWASWL DOWNLOAD MANAGER</text>`,
  ),
  save(
    "nsis-sidebar.bmp",
    164,
    314,
    `${brandIcon(26, 24, 112)}<path d="M37 164h90" stroke="${cyan}" stroke-width="2"/><text x="82" y="190" text-anchor="middle" fill="#ffffff" font-family="Segoe UI, sans-serif" font-size="12" font-weight="600">TDM</text><text x="82" y="209" text-anchor="middle" fill="${cyan}" font-family="Segoe UI, sans-serif" font-size="8" letter-spacing="1">DOWNLOAD MANAGER</text><text x="82" y="279" text-anchor="middle" fill="#b8d7e7" font-family="Segoe UI, sans-serif" font-size="8">TDM</text>`,
  ),
  save(
    "wix-banner.bmp",
    493,
    58,
    `${brandIcon(12, 6, 46)}<text x="72" y="27" fill="#ffffff" font-family="Segoe UI, sans-serif" font-size="18" font-weight="700">TDM</text><text x="73" y="44" fill="${cyan}" font-family="Segoe UI, sans-serif" font-size="9" letter-spacing="2">DOWNLOAD WITH CONFIDENCE</text>`,
  ),
  save(
    "wix-dialog.bmp",
    493,
    312,
    `${brandIcon(169, 25, 155)}<path d="M173 194h146" stroke="${cyan}" stroke-width="3"/><text x="246" y="224" text-anchor="middle" fill="#ffffff" font-family="Segoe UI, sans-serif" font-size="16" font-weight="600">TDM</text><text x="246" y="252" text-anchor="middle" fill="#b8d7e7" font-family="Segoe UI, sans-serif" font-size="10">TDM</text>`,
  ),
]);

console.log(`Created four branded installer bitmaps in ${output}`);
