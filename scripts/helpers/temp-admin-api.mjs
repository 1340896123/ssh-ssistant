import { mkdir, mkdtemp, writeFile } from "node:fs/promises";
import { spawn, spawnSync } from "node:child_process";
import path from "node:path";
import os from "node:os";
import net from "node:net";

const ADMIN_API_PROJECT = path.resolve("backend", "SshAssistant.AdminApi", "SshAssistant.AdminApi.csproj");

function normalizeBaseUrl(url) {
  return String(url || "").replace(/\/+$/, "");
}

export async function requestJson(baseUrl, pathName, options = {}) {
  const response = await fetch(`${normalizeBaseUrl(baseUrl)}${pathName}`, {
    ...options,
    headers: {
      "Content-Type": "application/json",
      ...(options.headers || {}),
    },
  });

  let payload = null;
  const text = await response.text();
  if (text) {
    try {
      payload = JSON.parse(text);
    } catch {
      payload = text;
    }
  }

  if (!response.ok) {
    const detail =
      payload && typeof payload === "object" && "error" in payload
        ? payload.error
        : JSON.stringify(payload);
    throw new Error(`${response.status} ${response.statusText}: ${detail}`);
  }

  return payload;
}

export function requestJsonAllowError(baseUrl, pathName, options = {}) {
  return fetch(`${normalizeBaseUrl(baseUrl)}${pathName}`, {
    ...options,
    headers: {
      "Content-Type": "application/json",
      ...(options.headers || {}),
    },
  }).then(async (response) => {
    let payload = null;
    const text = await response.text();
    if (text) {
      try {
        payload = JSON.parse(text);
      } catch {
        payload = text;
      }
    }

    return {
      ok: response.ok,
      status: response.status,
      statusText: response.statusText,
      payload,
    };
  });
}

export async function waitForHttp(baseUrl, pathName = "/api/admin/dashboard", timeoutMs = 20000) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < timeoutMs) {
    try {
      const response = await fetch(`${normalizeBaseUrl(baseUrl)}${pathName}`);
      if (response.status > 0) {
        return;
      }
    } catch {
      // Ignore until the server is ready.
    }

    await new Promise((resolve) => setTimeout(resolve, 300));
  }

  throw new Error(`Timed out waiting for ${baseUrl}${pathName}.`);
}

export async function buildAdminApi(outputDir) {
  await mkdir(outputDir, { recursive: true });
  let lastError = null;

  for (let attempt = 1; attempt <= 3; attempt += 1) {
    const result = spawnSync(
      "dotnet",
      [
        "build",
        ADMIN_API_PROJECT,
        "--disable-build-servers",
        "-o",
        outputDir,
        "-p:UseSharedCompilation=false",
      ],
      {
        cwd: path.resolve("."),
        encoding: "utf8",
        windowsHide: true,
        maxBuffer: 1024 * 1024 * 20,
      },
    );

    if (result.status === 0) {
      return {
        stdout: result.stdout || "",
        stderr: result.stderr || "",
      };
    }

    const output = `${result.stderr || ""}\n${result.stdout || ""}\n${result.error || ""}`;
    lastError = output.trim() || "unknown error";

    const isTransientLockError =
      output.includes("being used by another process") ||
      output.includes("MSB3501");

    if (!isTransientLockError || attempt === 3) {
      break;
    }

    await new Promise((resolve) => setTimeout(resolve, attempt * 1000));
  }

  throw new Error(`dotnet build failed: ${lastError}`);
}

export async function startTempAdminApi({
  port = 5061,
  outputRoot = path.resolve(".tmp"),
  label = "personal-register",
} = {}) {
  const baseTempDir = await mkdtemp(path.join(outputRoot, `${label}-`));
  const outputDir = path.join(baseTempDir, "app");
  const logsDir = path.join(baseTempDir, "logs");
  await mkdir(logsDir, { recursive: true });

  await buildAdminApi(outputDir);

  const stdoutChunks = [];
  const stderrChunks = [];
  const resolvedPort = await ensureAvailablePort(port);
  const baseUrl = `http://127.0.0.1:${resolvedPort}`;
  const server = spawn("dotnet", ["SshAssistant.AdminApi.dll", "--urls", baseUrl], {
    cwd: outputDir,
    env: {
      ...process.env,
      ASPNETCORE_ENVIRONMENT: "Production",
    },
    stdio: ["ignore", "pipe", "pipe"],
    windowsHide: true,
  });

  server.stdout.on("data", (chunk) => {
    stdoutChunks.push(chunk.toString());
  });
  server.stderr.on("data", (chunk) => {
    stderrChunks.push(chunk.toString());
  });

  await waitForHttp(baseUrl);

  async function stop() {
    if (!server.killed) {
      server.kill();
      await new Promise((resolve) => setTimeout(resolve, 500));
      if (!server.killed) {
        server.kill("SIGKILL");
      }
    }

    await writeFile(path.join(logsDir, "stdout.log"), stdoutChunks.join(""), "utf8");
    await writeFile(path.join(logsDir, "stderr.log"), stderrChunks.join(""), "utf8");
  }

  return {
    baseUrl,
    outputDir,
    tempDir: baseTempDir,
    logsDir,
    process: server,
    stop,
  };
}

export function nextPort(start = 5061) {
  return start + Math.floor(Math.random() * 200);
}

export function ensureAvailablePort(port) {
  return new Promise((resolve, reject) => {
    const tester = net.createServer();
    tester.unref();
    tester.once("error", reject);
    tester.listen(port, "127.0.0.1", () => {
      const address = tester.address();
      tester.close((closeError) => {
        if (closeError) {
          reject(closeError);
          return;
        }
        if (typeof address === "object" && address?.port) {
          resolve(address.port);
          return;
        }
        resolve(port);
      });
    });
  });
}

export async function findAvailablePort(start = 5061, attempts = 12, step = 1) {
  let lastError = null;

  for (let attempt = 0; attempt < attempts; attempt += 1) {
    const candidate = start + attempt * step;
    try {
      return await ensureAvailablePort(candidate);
    } catch (error) {
      lastError = error;
    }
  }

  throw lastError || new Error(`Unable to find available port near ${start}.`);
}
