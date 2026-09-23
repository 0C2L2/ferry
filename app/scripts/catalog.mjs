// Ferry's Linux app catalog (linux-equivalents.json): check it, and find
// candidates for new entries.
//
//   node scripts/catalog.mjs verify              every entry is well-formed and
//                                                every automatic install exists
//                                                on Ubuntu 24.04
//   node scripts/catalog.mjs find Discord "OBS"  Snap Store + Ubuntu archive
//                                                candidates, for drafting entries
//
// Adding apps: `find` the candidates, draft the entry (by hand or with Claude
// from the find output), have a person review the diff, and `verify` must pass.
// Nothing drafted by AI ships unreviewed.
//
// `verify` enforces what ferry-restore relies on when it installs as root:
// the packages exist, and a snap comes from a verified or Canonical-vetted
// ("starred") publisher, or the vendor account pinned in the entry's
// `snap_publisher` — a look-alike snap is the easiest way to ship malware.

import { existsSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { gunzipSync } from "node:zlib";

const CATALOG = join(dirname(fileURLToPath(import.meta.url)), "..", "linux-equivalents.json");
const KINDS = ["same", "alternative", "none", "builtin"];
const TRUSTED = ["verified", "starred"];
const SNAP = "https://api.snapcraft.io/v2/snaps";
const SNAP_HEADERS = { "Snap-Device-Series": "16" };

// Same grammar as parse_hint / valid_package in src-tauri/src/ubuntu/apps.rs.
const PKG = /^[a-z0-9][a-z0-9+.-]{0,63}$/;
export function parseHint(hint) {
  const w = hint.split(/\s+/).filter(Boolean);
  if (w[0] !== "sudo" || w[2] !== "install") return null;
  if (w[1] === "apt" && w.length > 3 && w.slice(3).every(p => PKG.test(p))) {
    return { manager: "apt", packages: w.slice(3), classic: false };
  }
  if (w[1] === "snap" && PKG.test(w[3] ?? "") && (w.length === 4 || (w.length === 5 && w[4] === "--classic"))) {
    return { manager: "snap", packages: [w[3]], classic: w.length === 5 };
  }
  return null;
}

/** Every binary package in Ubuntu 24.04 (release + updates, all components),
 *  name → "component: description". Cached for a day; ~21 MB to download. */
async function aptIndex() {
  const cache = join(tmpdir(), "ferry-noble-packages.json");
  if (existsSync(cache) && Date.now() - statSync(cache).mtimeMs < 86_400_000) {
    return new Map(JSON.parse(readFileSync(cache, "utf8")));
  }
  const index = new Map();
  for (const suite of ["noble", "noble-updates"]) {
    for (const component of ["main", "restricted", "universe", "multiverse"]) {
      const url = `http://archive.ubuntu.com/ubuntu/dists/${suite}/${component}/binary-amd64/Packages.gz`;
      const res = await fetch(url);
      if (!res.ok) throw new Error(`${url}: HTTP ${res.status}`);
      const text = gunzipSync(Buffer.from(await res.arrayBuffer())).toString("utf8");
      for (const stanza of text.split("\n\n")) {
        const name = /^Package: (.+)$/m.exec(stanza)?.[1];
        if (name && !index.has(name)) index.set(name, `${component}: ${/^Description: (.+)$/m.exec(stanza)?.[1] ?? ""}`);
      }
    }
  }
  writeFileSync(cache, JSON.stringify([...index]));
  return index;
}

/** Publisher and stable amd64 confinement of a snap; null if it doesn't exist. */
async function snapInfo(name, attempt = 0) {
  const res = await fetch(`${SNAP}/info/${name}?fields=publisher,confinement`, { headers: SNAP_HEADERS });
  // The store rate-limits bursts: wait as told (or back off) and try again.
  if (res.status === 429 && attempt < 5) {
    const wait = Number(res.headers.get("retry-after")) * 1000 || 2000 * 2 ** attempt;
    await new Promise(r => setTimeout(r, wait));
    return snapInfo(name, attempt + 1);
  }
  if (res.status === 404) return null;
  if (!res.ok) throw new Error(`Snap Store: HTTP ${res.status} for ${name}`);
  const d = await res.json();
  const track = d["default-track"] || "latest";
  const stable = d["channel-map"].find(
    c => c.channel.architecture === "amd64" && c.channel.risk === "stable" && c.channel.track === track,
  );
  return { publisher: d.snap.publisher.username, validation: d.snap.publisher.validation, confinement: stable?.confinement };
}

async function pool(items, size, fn) {
  const out = new Array(items.length);
  let next = 0;
  await Promise.all(
    Array.from({ length: size }, async () => {
      while (next < items.length) {
        const i = next++;
        out[i] = await fn(items[i]);
      }
    }),
  );
  return out;
}

async function verify() {
  const entries = JSON.parse(readFileSync(CATALOG, "utf8"));
  const errors = [];
  const bad = (e, msg) => errors.push(`${e.windows_name}: ${msg}`);

  const names = new Map();
  for (const e of entries) {
    if (!KINDS.includes(e.kind)) bad(e, `unknown kind "${e.kind}"`);
    for (const key of [e.windows_name, ...(e.aliases ?? [])]) {
      const lower = String(key ?? "").toLowerCase();
      if (!lower) bad(e, "empty name");
      else if (names.has(lower)) bad(e, `"${key}" is also used by ${names.get(lower)}`);
      names.set(lower, e.windows_name);
    }
    if (e.kind !== "none" && !e.linux_name) bad(e, "missing linux_name");
    if (e.kind !== "same" && !e.note) bad(e, `a "${e.kind}" entry needs a note saying what changes`);
    if (e.kind === "none" && e.install) bad(e, `a "none" entry can't have an install`);
    if (e.install.startsWith("sudo") && !parseHint(e.install)) bad(e, `install hint doesn't parse: ${e.install}`);
    if (e.snap_publisher && parseHint(e.install)?.manager !== "snap") bad(e, "snap_publisher is set but the install isn't a snap");
  }

  const auto = entries.map(e => [e, parseHint(e.install)]).filter(([, h]) => h);
  const apt = await aptIndex();
  for (const [e, h] of auto.filter(([, h]) => h.manager === "apt")) {
    for (const p of h.packages) if (!apt.has(p)) bad(e, `apt package "${p}" isn't in Ubuntu 24.04`);
  }
  const snaps = auto.filter(([, h]) => h.manager === "snap");
  const infos = await pool(snaps, 4, ([, h]) => snapInfo(h.packages[0]));
  snaps.forEach(([e, h], i) => {
    const s = infos[i];
    const name = h.packages[0];
    if (!s) return bad(e, `snap "${name}" doesn't exist`);
    // A vendor's own unbadged account can be pinned by name after a human
    // checked it; a pin is stricter than a badge, so it replaces the check.
    if (e.snap_publisher ? s.publisher !== e.snap_publisher : !TRUSTED.includes(s.validation)) {
      bad(
        e,
        e.snap_publisher
          ? `snap "${name}" is now published by ${s.publisher}, not the pinned ${e.snap_publisher}`
          : `snap "${name}" is published by ${s.publisher} (${s.validation}): only verified or starred publishers, or a pinned snap_publisher, install automatically`,
      );
    }
    if (!s.confinement) return bad(e, `snap "${name}" has no stable amd64 release`);
    if ((s.confinement === "classic") !== h.classic) {
      bad(e, `snap "${name}" is ${s.confinement}: the hint ${h.classic ? "must not" : "must"} use --classic`);
    }
  });

  console.log(`${entries.length} entries, ${auto.length} automatic installs checked against Ubuntu 24.04.`);
  if (errors.length) {
    console.error(errors.map(e => `  ✗ ${e}`).join("\n"));
    process.exit(1);
  }
  console.log("All good.");
}

async function find(queries) {
  const apt = await aptIndex();
  for (const q of queries) {
    console.log(`\n${q}`);
    const res = await fetch(`${SNAP}/find?q=${encodeURIComponent(q)}&fields=title,publisher,summary`, { headers: SNAP_HEADERS });
    for (const s of res.ok ? ((await res.json()).results ?? []).slice(0, 5) : []) {
      const p = s.snap.publisher;
      console.log(`  snap ${s.name.padEnd(26)} ${`${p.username} (${p.validation})`.padEnd(34)} ${s.snap.summary ?? ""}`.slice(0, 150));
    }
    const needle = q.toLowerCase().replace(/[^a-z0-9]/g, "");
    for (const [name, desc] of [...apt].filter(([n]) => n.replace(/[^a-z0-9]/g, "").includes(needle)).slice(0, 6)) {
      console.log(`  apt  ${name.padEnd(26)} ${desc}`.slice(0, 150));
    }
  }
}

const [command, ...args] = process.argv.slice(2);
if (command === "verify") await verify();
else if (command === "find" && args.length) await find(args);
else {
  console.log('usage: node scripts/catalog.mjs verify | find "App name"...');
  process.exit(2);
}
