import { execFile } from "node:child_process";
import { pathToFileURL } from "node:url";
import { existsSync, readdirSync } from "node:fs";
import path from "node:path";

const BROWSER_CLOSE_TIMEOUT_MS = 5000;
const wrappedModules = new WeakSet();
const wrappedBrowserTypes = new WeakSet();
const wrappedBrowsers = new WeakSet();

function delay(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function execFileAsync(file, args) {
  return new Promise((resolve, reject) => {
    execFile(file, args, (error, stdout, stderr) => {
      if (error) {
        reject(error);
        return;
      }
      resolve({ stdout, stderr });
    });
  });
}

async function killChildBrowserProcessesOfCurrentNode() {
  if (process.platform !== "win32") {
    return;
  }

  const psScript = [
    "$ErrorActionPreference = 'SilentlyContinue'",
    `$parentPid = ${process.pid}`,
    "$names = @('chrome.exe','msedge.exe','chromium.exe','chrome-headless-shell.exe')",
    "Get-CimInstance Win32_Process |",
    "  Where-Object { $_.ParentProcessId -eq $parentPid -and $names -contains $_.Name } |",
    "  ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }",
  ].join("; ");

  await execFileAsync("powershell", [
    "-NoProfile",
    "-NonInteractive",
    "-Command",
    psScript,
  ]).catch(() => {});
}

function wrapBrowserInstance(browser) {
  if (!browser || wrappedBrowsers.has(browser)) {
    return browser;
  }

  const originalClose = browser.close.bind(browser);
  browser.close = async (...args) => {
    const contexts = typeof browser.contexts === "function" ? browser.contexts() : [];
    await Promise.allSettled(
      contexts.map(async (context) => {
        await Promise.race([context.close(), delay(2000)]);
      }),
    );

    const closePromise = originalClose(...args);
    const didTimeout = await Promise.race([
      closePromise.then(() => false).catch(() => false),
      delay(BROWSER_CLOSE_TIMEOUT_MS).then(() => true),
    ]);

    if (didTimeout) {
      await killChildBrowserProcessesOfCurrentNode();
      await Promise.race([closePromise.catch(() => {}), delay(1000)]);
      return;
    }

    await closePromise.catch(() => {});
  };

  wrappedBrowsers.add(browser);
  return browser;
}

function wrapBrowserType(browserType) {
  if (!browserType || wrappedBrowserTypes.has(browserType)) {
    return browserType;
  }

  const originalLaunch = browserType.launch.bind(browserType);
  browserType.launch = async (...args) => {
    const browser = await originalLaunch(...args);
    return wrapBrowserInstance(browser);
  };

  wrappedBrowserTypes.add(browserType);
  return browserType;
}

function wrapPlaywrightModule(mod) {
  if (!mod || wrappedModules.has(mod)) {
    return mod;
  }

  for (const key of ["chromium", "firefox", "webkit"]) {
    if (mod[key]) {
      wrapBrowserType(mod[key]);
    }
  }

  wrappedModules.add(mod);
  return mod;
}

function candidatesFromEnv() {
  const explicit = process.env.PLAYWRIGHT_MODULE?.trim();
  return explicit ? [explicit] : [];
}

function candidatesFromWorkspace() {
  const workspaceRoot = process.cwd();
  const candidates = [];

  const localToolingPlaywrightRoot = path.join(
    workspaceRoot,
    ".tooling",
    "playwright-runtime",
    "playwright",
  );
  candidates.push(pathToFileURL(path.join(localToolingPlaywrightRoot, "index.mjs")).href);
  candidates.push(pathToFileURL(path.join(localToolingPlaywrightRoot, "index.js")).href);

  const localPlaywrightRoot = path.join(workspaceRoot, "node_modules", "playwright");
  candidates.push(pathToFileURL(path.join(localPlaywrightRoot, "index.mjs")).href);
  candidates.push(pathToFileURL(path.join(localPlaywrightRoot, "index.js")).href);

  return candidates;
}

function npmRootCandidates() {
  const appData = process.env.APPDATA?.trim();
  const npmConfigPrefix = process.env.npm_config_prefix?.trim();
  const nodePath = process.env.NODE_PATH?.trim();

  return [
    nodePath,
    npmConfigPrefix ? path.join(npmConfigPrefix, "node_modules") : null,
    appData ? path.join(appData, "npm", "node_modules") : null,
  ].filter(Boolean);
}

function candidatesFromGlobalRoot() {
  const moduleRoots = npmRootCandidates();
  const candidates = [];

  for (const root of moduleRoots) {
    candidates.push(pathToFileURL(path.join(root, "playwright", "index.mjs")).href);
    candidates.push(pathToFileURL(path.join(root, "playwright", "index.js")).href);
  }

  return candidates;
}

async function importFirstAvailable(candidates) {
  let lastError = null;

  for (const candidate of candidates) {
    try {
      const mod = wrapPlaywrightModule(await import(candidate));
      return {
        module: mod,
        source: candidate,
      };
    } catch (error) {
      lastError = error;
    }
  }

  throw new Error(
    `Unable to load Playwright runtime. Tried: ${candidates.join(", ")}. Last error: ${String(
      lastError?.message || lastError,
    )}`,
  );
}

export async function loadPlaywright() {
  const candidates = [
    ...candidatesFromEnv(),
    ...candidatesFromWorkspace(),
    ...candidatesFromGlobalRoot(),
  ];
  return importFirstAvailable(candidates);
}

function compareRevisionDesc(a, b) {
  const aRevision = Number(a.match(/-(\d+)$/)?.[1] || 0);
  const bRevision = Number(b.match(/-(\d+)$/)?.[1] || 0);
  return bRevision - aRevision;
}

function playwrightBrowserRoot() {
  const workspaceToolingRoot = path.join(
    process.cwd(),
    ".tooling",
    "playwright-runtime",
    "ms-playwright",
  );
  if (existsSync(workspaceToolingRoot)) {
    return workspaceToolingRoot;
  }

  const localAppData = process.env.LOCALAPPDATA?.trim();
  return localAppData ? path.join(localAppData, "ms-playwright") : null;
}

function resolveBrowserBinaryFromDisk() {
  const root = playwrightBrowserRoot();
  if (!root || !existsSync(root)) {
    return null;
  }

  const entries = readdirSync(root, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => entry.name)
    .sort(compareRevisionDesc);

  for (const entry of entries) {
    const headlessShell = path.join(
      root,
      entry,
      "chrome-headless-shell-win64",
      "chrome-headless-shell.exe",
    );
    if (entry.startsWith("chromium_headless_shell-") && existsSync(headlessShell)) {
      return headlessShell;
    }
  }

  for (const entry of entries) {
    const chromeBinary = path.join(root, entry, "chrome-win64", "chrome.exe");
    if (entry.startsWith("chromium-") && existsSync(chromeBinary)) {
      return chromeBinary;
    }
  }

  return null;
}

export function ensureChromiumExecutable(chromium) {
  const preferredPath = chromium.executablePath();
  if (preferredPath && existsSync(preferredPath)) {
    return preferredPath;
  }

  const resolvedPath = resolveBrowserBinaryFromDisk();
  if (resolvedPath) {
    return resolvedPath;
  }

  throw new Error(
    `No usable Chromium executable was found. Playwright preferred path: ${preferredPath}`,
  );
}

export function chromiumLaunchOptions(chromium, options = {}) {
  return {
    ...options,
    executablePath: ensureChromiumExecutable(chromium),
  };
}
