import { spawn } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";

const checks = [
  ["verify:web:switch", "登录切换"],
  ["verify:web:register", "个人注册"],
  ["verify:web:settings", "设置面板"],
  ["verify:web:settings-save", "设置保存"],
  ["verify:web:asset-create", "资产创建"],
  ["verify:web:asset-edit", "资产编辑保存"],
  ["verify:web:asset-delete", "资产删除"],
  ["verify:web:connection-test", "连接测试"],
  ["verify:web:ops-console", "Ops Console"],
  ["verify:web:ops-jobs", "Ops Jobs"],
  ["verify:web:file-editor", "文件编辑器打开"],
  ["verify:web:file-editor-save", "文件保存"],
  ["verify:web:file-editor-close-confirm", "未保存关闭确认"],
  ["verify:web:file-rename", "文件重命名"],
  ["verify:web:file-delete-confirm", "文件删除确认"],
  ["verify:web:file-upload", "文件上传"],
  ["verify:web:file-download", "文件下载"],
  ["verify:web:tunnel-delete", "隧道删除"],
  ["verify:web:tunnel-create", "隧道创建"],
  ["verify:web:tunnel-lifecycle", "隧道启停"],
  ["verify:web:sessions-lifecycle", "会话断开重连"],
];

const STATUS_FILE = path.resolve("tmp", "verify-web-status.json");
const BUILD_TIMEOUT_MS = 5 * 60 * 1000;
const SCRIPT_TIMEOUT_MS = 3 * 60 * 1000;
const WEB_STEP_RETRY_LIMIT = 1;
const STEP_SETTLE_MS = 1500;
const RETRY_SETTLE_MS = 2500;

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

function runNpm(
  args,
  label,
  scriptName = args.join(" "),
  timeoutMs = SCRIPT_TIMEOUT_MS,
) {
  return new Promise((resolve) => {
    const startedAt = Date.now();
    let resolved = false;
    const child =
      process.platform === "win32"
        ? spawn("cmd", ["/c", "npm", ...args], {
            cwd: process.cwd(),
            stdio: "inherit",
            shell: false,
          })
        : spawn("npm", args, {
            cwd: process.cwd(),
            stdio: "inherit",
            shell: false,
          });

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
  const skipBuild = process.env.SSH_ASSISTANT_SKIP_BUILD === "1";
  const startedAt = Date.now();

  if (!skipBuild) {
    console.log(`[verify:web] Running build...`);
    const buildResult = await runNpm(
      ["run", "build"],
      "构建",
      "build",
      BUILD_TIMEOUT_MS,
    );
    results.push(buildResult);
    await writeStatus({
      ok: false,
      phase: "build",
      startedAt,
      updatedAt: Date.now(),
      total: checks.length + 1,
      executed: results.length,
      results,
    });
    if (!buildResult.ok) {
      const payload = {
        ok: false,
        total: checks.length + 1,
        executed: results.length,
        failedScript: buildResult.scriptName,
        results,
      };
      await writeStatus({
        ...payload,
        phase: "failed",
        startedAt,
        updatedAt: Date.now(),
      });
      console.log(JSON.stringify(payload, null, 2));
      process.exitCode = 1;
      return;
    }
  }

  for (const [scriptName, label] of checks) {
    console.log(`[verify:web] Running ${scriptName} (${label})...`);
    const attempts = [];
    let finalResult = await runNpm(["run", scriptName], label, scriptName);
    attempts.push(finalResult);

    for (let retryIndex = 0; !finalResult.ok && retryIndex < WEB_STEP_RETRY_LIMIT; retryIndex += 1) {
      await delay(RETRY_SETTLE_MS);
      console.log(
        `[verify:web] Retrying ${scriptName} (${retryIndex + 1}/${WEB_STEP_RETRY_LIMIT})...`,
      );
      finalResult = await runNpm(
        ["run", scriptName],
        `${label}（重试 ${retryIndex + 1}/${WEB_STEP_RETRY_LIMIT}）`,
        scriptName,
      );
      attempts.push(finalResult);
    }

    if (attempts.length > 1) {
      finalResult = {
        ...finalResult,
        attempts,
      };
    }

    results.push(finalResult);
    await writeStatus({
      ok: false,
      phase: scriptName,
      startedAt,
      updatedAt: Date.now(),
      total: checks.length + (skipBuild ? 0 : 1),
      executed: results.length,
      results,
    });
    if (!finalResult.ok) {
      break;
    }

    await delay(STEP_SETTLE_MS);
  }

  const failed = results.find((item) => !item.ok) ?? null;
  const payload = {
    ok: !failed,
    total: checks.length + (skipBuild ? 0 : 1),
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
