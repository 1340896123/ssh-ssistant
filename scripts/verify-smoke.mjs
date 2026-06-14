import { spawn } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";

const steps = [
  ["build", "构建"],
  ["verify:enterprise", "企业链路"],
  ["verify:g3", "账单与 AI 计量"],
  ["verify:g4", "多模式主回归"],
  ["verify:web", "Web UI 回归套件"],
];

const STATUS_FILE = path.resolve("tmp", "verify-smoke-status.json");
const BUILD_TIMEOUT_MS = 5 * 60 * 1000;
const STEP_TIMEOUT_MS = 12 * 60 * 1000;
const SMOKE_RETRYABLE = new Set(["verify:g3", "verify:web"]);
const STEP_SETTLE_MS = 2000;
const RETRY_SETTLE_MS = 3000;

async function writeStatus(payload) {
  await mkdir(path.dirname(STATUS_FILE), { recursive: true });
  await writeFile(STATUS_FILE, `${JSON.stringify(payload, null, 2)}\n`, "utf8");
}

function delay(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function killProcessTree(child) {
  if (!child?.pid) return Promise.resolve();

  if (process.platform === "win32") {
    return new Promise((resolve) => {
      const killer = spawn(
        "taskkill",
        ["/PID", String(child.pid), "/T", "/F"],
        {
          stdio: "ignore",
          shell: false,
        },
      );
      killer.on("exit", () => resolve());
      killer.on("error", () => resolve());
    });
  }

  child.kill("SIGTERM");
  return Promise.resolve();
}

function runNpmScript(scriptName, label, env = {}) {
  return new Promise((resolve) => {
    const startedAt = Date.now();
    let resolved = false;
    const child =
      process.platform === "win32"
        ? spawn("cmd", ["/c", "npm", "run", scriptName], {
            cwd: process.cwd(),
            stdio: "inherit",
            shell: false,
            env: {
              ...process.env,
              ...env,
            },
          })
        : spawn("npm", ["run", scriptName], {
            cwd: process.cwd(),
            stdio: "inherit",
            shell: false,
            env: {
              ...process.env,
              ...env,
            },
          });

    const timeoutMs = scriptName === "build" ? BUILD_TIMEOUT_MS : STEP_TIMEOUT_MS;

    const finish = (payload) => {
      if (resolved) return;
      resolved = true;
      clearTimeout(timer);
      resolve({
        scriptName,
        label,
        durationMs: Date.now() - startedAt,
        ...payload,
      });
    };

    const timer = setTimeout(async () => {
      await killProcessTree(child);
      finish({
        ok: false,
        code: -1,
        timedOut: true,
      });
    }, timeoutMs);

    child.on("error", (error) => {
      finish({
        ok: false,
        code: -1,
        error: String(error?.message || error),
      });
    });

    child.on("exit", (code) => {
      finish({
        ok: code === 0,
        code: code ?? -1,
      });
    });
  });
}

async function main() {
  const results = [];
  const startedAt = Date.now();

  for (const [scriptName, label] of steps) {
    console.log(`[verify:smoke] Running ${scriptName} (${label})...`);
    const attempts = [];
    let result = await runNpmScript(
      scriptName,
      label,
      scriptName === "verify:web" ? { SSH_ASSISTANT_SKIP_BUILD: "1" } : {},
    );
    attempts.push(result);

    if (!result.ok && SMOKE_RETRYABLE.has(scriptName)) {
      await delay(RETRY_SETTLE_MS);
      console.log(`[verify:smoke] Retrying ${scriptName}...`);
      result = await runNpmScript(
        scriptName,
        `${label}（重试）`,
        scriptName === "verify:web" ? { SSH_ASSISTANT_SKIP_BUILD: "1" } : {},
      );
      attempts.push(result);
    }

    if (attempts.length > 1) {
      result = {
        ...result,
        attempts,
      };
    }

    results.push(result);
    await writeStatus({
      ok: false,
      phase: scriptName,
      startedAt,
      updatedAt: Date.now(),
      total: steps.length,
      executed: results.length,
      results,
    });
    if (!result.ok) {
      break;
    }

    await delay(STEP_SETTLE_MS);
  }

  const failed = results.find((item) => !item.ok) ?? null;
  const payload = {
    ok: !failed,
    total: steps.length,
    executed: results.length,
    failedScript: failed?.scriptName ?? null,
    results,
  };
  await writeStatus({
    ...payload,
    phase: failed ? "failed" : "complete",
    startedAt,
    updatedAt: Date.now(),
  });

  console.log(JSON.stringify(payload, null, 2));

  if (failed) {
    process.exitCode = 1;
  }
}

main().catch((error) => {
  console.error(
    JSON.stringify(
      {
        ok: false,
        error: String(error?.message || error),
      },
      null,
      2,
    ),
  );
  process.exitCode = 1;
});
