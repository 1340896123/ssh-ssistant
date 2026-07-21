// @ts-check
/**
 * Build/dev the Tauri app for a specific variant (local | personal).
 *
 * Sets the `VITE_APP_VARIANT` environment variable so the frontend can read it
 * via `import.meta.env`, then invokes the Tauri CLI with the matching config
 * overlay from `src-tauri/tauri.<variant>.conf.json` (merged on top of the base
 * `src-tauri/tauri.conf.json` by Tauri's `--config` flag).
 *
 * Usage:
 *   node scripts/build-variant.mjs dev local
 *   node scripts/build-variant.mjs build personal
 */
import { spawn } from "node:child_process";

const VALID_VARIANTS = new Set(["local", "personal"]);
const VALID_COMMANDS = new Set(["dev", "build"]);

function parseArgs(argv) {
  const command = argv[0];
  const variant = argv[1];
  if (!VALID_COMMANDS.has(command)) {
    console.error(
      `Invalid command "${command}". Expected one of: ${[...VALID_COMMANDS].join(", ")}.`,
    );
    process.exit(1);
  }
  if (!VALID_VARIANTS.has(variant)) {
    console.error(
      `Invalid variant "${variant}". Expected one of: ${[...VALID_VARIANTS].join(", ")}.`,
    );
    process.exit(1);
  }
  return { command, variant };
}

function main() {
  const { command, variant } = parseArgs(process.argv.slice(2));
  const configPath = `src-tauri/tauri.${variant}.conf.json`;

  // Forward extra args (e.g. -- --features foo) after variant.
  const passthrough = process.argv.slice(3);

  const child = spawn(
    "npx",
    ["tauri", command, "--config", configPath, ...passthrough],
    {
      stdio: "inherit",
      shell: process.platform === "win32",
      env: {
        ...process.env,
        VITE_APP_VARIANT: variant,
      },
    },
  );

  child.on("close", (code) => {
    process.exit(code ?? 0);
  });
}

main();
