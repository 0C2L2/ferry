// Builds the Linux `ferry-restore` inside Docker and drops it where Tauri
// bundles it from (src-tauri/resources/ferry-restore). The Windows app copies
// it onto the USB, so a Windows build machine needs this before `tauri dev`.
//
// Debian bookworm's glibc (2.36) is older than Ubuntu 24.04's (2.39), so the
// binary runs there. Named volumes keep the cargo cache between runs.
import { spawnSync } from "node:child_process";
import { mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const app = resolve(dirname(fileURLToPath(import.meta.url)), "..");
mkdirSync(resolve(app, "src-tauri", "resources"), { recursive: true });

const result = spawnSync(
  "docker",
  [
    "run", "--rm",
    "-v", `${app}:/app`,
    "-v", "ferry-cargo-registry:/usr/local/cargo/registry",
    "-v", "ferry-linux-target:/target",
    "-e", "CARGO_TARGET_DIR=/target",
    "-w", "/app/src-tauri",
    "rust:1-bookworm",
    "sh", "-c",
    "cargo build --locked --release --features restore-cli --bin ferry-restore && cp /target/release/ferry-restore resources/ferry-restore",
  ],
  { stdio: "inherit" },
);
process.exit(result.status ?? 1);
