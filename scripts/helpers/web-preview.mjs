import { createServer } from "node:http";
import { mkdir } from "node:fs/promises";
import { createReadStream, existsSync, statSync } from "node:fs";
import path from "node:path";
import { nextPort } from "./temp-admin-api.mjs";

const MIME_TYPES = new Map([
  [".html", "text/html; charset=utf-8"],
  [".js", "application/javascript; charset=utf-8"],
  [".css", "text/css; charset=utf-8"],
  [".json", "application/json; charset=utf-8"],
  [".svg", "image/svg+xml"],
  [".png", "image/png"],
  [".jpg", "image/jpeg"],
  [".jpeg", "image/jpeg"],
  [".ico", "image/x-icon"],
  [".ttf", "font/ttf"],
  [".woff", "font/woff"],
  [".woff2", "font/woff2"],
]);

export async function waitForWebApp(url, timeoutMs = 30000) {
  const startedAt = Date.now();
  let lastError = null;
  while (Date.now() - startedAt < timeoutMs) {
    try {
      const response = await fetch(url);
      if (response.ok) {
        return;
      }
      lastError = new Error(`HTTP ${response.status}`);
    } catch (error) {
      lastError = error;
    }

    await new Promise((resolve) => setTimeout(resolve, 500));
  }

  throw new Error(
    `Timed out waiting for frontend app at ${url}. Last check: ${String(
      lastError?.message || lastError || "unknown",
    )}`,
  );
}

function contentTypeFor(filePath) {
  return MIME_TYPES.get(path.extname(filePath).toLowerCase()) || "application/octet-stream";
}

function delay(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function resolveDistFile(distRoot, requestUrl) {
  const url = new URL(requestUrl, "http://127.0.0.1");
  const pathname = decodeURIComponent(url.pathname);
  const normalizedPath = pathname === "/" ? "/index.html" : pathname;
  const candidate = path.normalize(path.join(distRoot, normalizedPath));
  if (!candidate.startsWith(path.normalize(distRoot))) {
    return null;
  }

  if (existsSync(candidate) && statSync(candidate).isFile()) {
    return candidate;
  }

  return null;
}

export async function startPreviewServer({
  port = 4173,
  outputRoot = path.resolve(".tmp"),
  label = "web-preview",
} = {}) {
  const distRoot = path.resolve("dist");
  if (!existsSync(distRoot)) {
    throw new Error(`Build output not found at ${distRoot}. Run \`npm run build\` first.`);
  }

  const logsDir = path.join(outputRoot, label);
  await mkdir(logsDir, { recursive: true });

  const sockets = new Set();

  const createStaticServer = () => {
    const httpServer = createServer((req, res) => {
      const filePath = resolveDistFile(distRoot, req.url || "/");
      if (!filePath) {
        res.statusCode = 404;
        res.end("Not Found");
        return;
      }

      res.setHeader("Content-Type", contentTypeFor(filePath));
      createReadStream(filePath).pipe(res);
    });

    httpServer.keepAliveTimeout = 0;
    httpServer.headersTimeout = 5000;
    httpServer.requestTimeout = 30000;
    httpServer.on("connection", (socket) => {
      sockets.add(socket);
      socket.on("close", () => sockets.delete(socket));
    });

    return httpServer;
  };

  let resolvedPort = port;
  let server = null;
  let lastError = null;

  for (let attempt = 0; attempt < 8; attempt += 1) {
    server = createStaticServer();
    try {
      await new Promise((resolve, reject) => {
        server.once("error", reject);
        server.listen(resolvedPort, "127.0.0.1", () => {
          server.off("error", reject);
          server.unref?.();
          resolve(undefined);
        });
      });
      lastError = null;
      break;
    } catch (error) {
      lastError = error;
      server.close();
      server = null;
      resolvedPort = nextPort(resolvedPort + 1);
    }
  }

  if (!server) {
    throw new Error(
      `Failed to start preview server after multiple port attempts: ${String(
        lastError?.message || lastError || "unknown error",
      )}`,
    );
  }

  const baseUrl = `http://127.0.0.1:${resolvedPort}`;
  await waitForWebApp(baseUrl);

  async function stop() {
    server.closeIdleConnections?.();
    server.closeAllConnections?.();
    for (const socket of sockets) {
      socket.unref?.();
      socket.destroy();
    }

    await Promise.race([
      new Promise((resolve) => {
        server.close(() => resolve(undefined));
      }),
      delay(1000),
    ]);
  }

  return {
    baseUrl,
    port: resolvedPort,
    logsDir,
    stop,
  };
}
