// Publishes ../site (static files, served by the Worker in wrangler.site.toml),
// with the newest Windows installer copied in as /downloads/Ferry-Setup.exe —
// a stable link for the Download button. Build the installer first:
// `npm run tauri build` in app/.
import { copyFileSync, mkdirSync, readdirSync, statSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
const site = join(root, "site");
const nsis = join(root, "app", "src-tauri", "target", "release", "bundle", "nsis");

const installer = readdirSync(nsis)
  .filter(f => f.endsWith("-setup.exe"))
  .map(f => join(nsis, f))
  .sort((a, b) => statSync(b).mtimeMs - statSync(a).mtimeMs)[0];
if (!installer) throw new Error(`No installer in ${nsis} — run \`npm run tauri build\` in app/ first.`);
mkdirSync(join(site, "downloads"), { recursive: true });
copyFileSync(installer, join(site, "downloads", "Ferry-Setup.exe"));
console.log(`Installer: ${installer}`);

const { status } = spawnSync("npx", ["wrangler", "deploy", "--config", "wrangler.site.toml"], {
  cwd: join(root, "server"),
  stdio: "inherit",
  shell: true,
});
process.exit(status ?? 1);
