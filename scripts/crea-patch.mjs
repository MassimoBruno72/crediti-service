// Crea una patch firmata (.crpatch) dalla pagina src/index.html.
// Uso: node scripts/crea-patch.mjs [versione]
// Richiede TAURI_SIGNING_PRIVATE_KEY (testo) oppure TAURI_SIGNING_PRIVATE_KEY_PATH (file)
// e, se la chiave ha una password, TAURI_SIGNING_PRIVATE_KEY_PASSWORD.
import { readFileSync, writeFileSync, mkdirSync, mkdtempSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { tmpdir } from "node:os";
import { join } from "node:path";

const conf = JSON.parse(readFileSync("src-tauri/tauri.conf.json", "utf8"));
const versione = (process.argv[2] || conf.version).replace(/^v/, "");
if (!/^\d+\.\d+\.\d+$/.test(versione)) throw new Error(`Versione non valida: ${versione}`);

let html = readFileSync("src/index.html", "utf8");
if (html.includes('<meta name="patch-version"')) throw new Error("src/index.html contiene già un marcatore di patch");
if (!html.includes("<head>")) throw new Error("<head> non trovato in src/index.html");
html = html.replace("<head>", `<head><meta name="patch-version" content="${versione}">`);

const tmp = mkdtempSync(join(tmpdir(), "crpatch-"));
const file = join(tmp, "index.html");
writeFileSync(file, html);
execFileSync("npx", ["tauri", "signer", "sign", file], { stdio: ["ignore", "ignore", "inherit"], env: process.env });
const firma = readFileSync(file + ".sig", "utf8").trim();

mkdirSync("dist-patch", { recursive: true });
const out = `dist-patch/Crediti-Service-patch-${versione}.crpatch`;
writeFileSync(
  out,
  JSON.stringify({
    formato: "crediti-service-patch",
    versione,
    html_b64: Buffer.from(html, "utf8").toString("base64"),
    firma,
  }),
);
console.log("Patch creata:", out);
