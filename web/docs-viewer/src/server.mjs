import { lstat, open, realpath, stat } from "node:fs/promises";
import http from "node:http";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const appRoot = path.resolve(here, "..");

export async function createDocsViewerServer(options) {
  const dataRoot = await requireDirectory(options.dataRoot, "--data");
  const staticRoot = await requireDirectory(
    options.staticRoot ?? path.join(appRoot, "dist"),
    "static root",
  );

  return http.createServer(async (request, response) => {
    try {
      if (!request.url || request.method !== "GET") {
        sendText(response, 405, "Method not allowed");
        return;
      }
      const url = new URL(request.url, "http://docs-viewer.local");
      if (url.pathname.startsWith("/data/")) {
        await serveFile(response, dataRoot, url.pathname.slice("/data/".length));
        return;
      }
      const staticPath = url.pathname === "/" ? "index.html" : url.pathname.slice(1);
      await serveFile(response, staticRoot, staticPath);
    } catch (error) {
      if (error instanceof HttpError) {
        sendText(response, error.status, error.message);
      } else {
        sendText(response, 500, "Internal server error");
      }
    }
  });
}

export function parseCliArgs(argv) {
  let dataRoot = null;
  let listen = "127.0.0.1:4173";
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--data") {
      dataRoot = argv[++index];
    } else if (arg === "--listen") {
      listen = argv[++index];
    } else {
      throw new Error(`unknown argument '${arg}'`);
    }
  }
  if (!dataRoot) {
    throw new Error("missing required --data <dir>");
  }
  const [host, portText] = String(listen).split(":");
  const port = Number(portText);
  if (!host || !Number.isInteger(port) || port <= 0 || port > 65535) {
    throw new Error(`invalid --listen '${listen}', expected host:port`);
  }
  return { dataRoot, host, port };
}

async function requireDirectory(inputPath, label) {
  if (!inputPath) {
    throw new Error(`missing ${label}`);
  }
  const resolved = path.resolve(inputPath);
  const metadata = await stat(resolved).catch(() => null);
  if (!metadata || !metadata.isDirectory()) {
    throw new Error(`${label} '${inputPath}' is not a directory`);
  }
  return resolved;
}

async function serveFile(response, root, relativeUrlPath) {
  const filePath = resolveWithin(root, relativeUrlPath);
  const linkMetadata = await lstat(filePath).catch(() => null);
  if (!linkMetadata) {
    throw new HttpError(404, "Not found");
  }
  if (linkMetadata.isSymbolicLink()) {
    throw new HttpError(403, "Forbidden");
  }
  const metadata = await stat(filePath).catch(() => null);
  if (!metadata || !metadata.isFile()) {
    throw new HttpError(404, "Not found");
  }
  await ensureRealPathWithinRoot(root, filePath);
  const file = await openFileForRead(filePath);
  response.writeHead(200, {
    "Content-Length": metadata.size,
    "Content-Type": contentType(filePath),
    "X-Content-Type-Options": "nosniff",
  });
  const stream = file.createReadStream({ autoClose: true });
  stream.on("error", () => response.destroy());
  stream.pipe(response);
}

async function ensureRealPathWithinRoot(root, filePath) {
  const [realRoot, realFile] = await Promise.all([realpath(root), realpath(filePath)]);
  const relative = path.relative(realRoot, realFile);
  if (relative.startsWith("..") || path.isAbsolute(relative)) {
    throw new HttpError(403, "Forbidden");
  }
}

async function openFileForRead(filePath) {
  try {
    return await open(filePath, "r");
  } catch (error) {
    if (error?.code === "EACCES" || error?.code === "EPERM") {
      throw new HttpError(403, "Forbidden");
    }
    throw new HttpError(404, "Not found");
  }
}

export function resolveWithin(root, relativeUrlPath) {
  let decoded;
  try {
    decoded = decodeURIComponent(relativeUrlPath);
  } catch {
    throw new HttpError(400, "Invalid path");
  }
  if (decoded.includes("\0") || path.isAbsolute(decoded)) {
    throw new HttpError(400, "Invalid path");
  }
  const resolved = path.resolve(root, decoded);
  const relative = path.relative(root, resolved);
  if (relative.startsWith("..") || path.isAbsolute(relative)) {
    throw new HttpError(403, "Forbidden");
  }
  return resolved;
}

function contentType(filePath) {
  const ext = path.extname(filePath);
  if (ext === ".html") {
    return "text/html; charset=utf-8";
  }
  if (ext === ".css") {
    return "text/css; charset=utf-8";
  }
  if (ext === ".js") {
    return "text/javascript; charset=utf-8";
  }
  if (ext === ".json") {
    return "application/json; charset=utf-8";
  }
  if (ext === ".md") {
    return "text/markdown; charset=utf-8";
  }
  return "application/octet-stream";
}

function sendText(response, status, message) {
  response.writeHead(status, {
    "Content-Type": "text/plain; charset=utf-8",
    "X-Content-Type-Options": "nosniff",
  });
  response.end(message);
}

class HttpError extends Error {
  constructor(status, message) {
    super(message);
    this.status = status;
  }
}

if (import.meta.url === `file://${process.argv[1]}`) {
  try {
    const { dataRoot, host, port } = parseCliArgs(process.argv.slice(2));
    const server = await createDocsViewerServer({ dataRoot });
    server.listen(port, host, () => {
      console.log(`docs viewer listening on http://${host}:${port}/`);
      console.log(`data root: ${path.resolve(dataRoot)}`);
    });
  } catch (error) {
    console.error(error.message);
    process.exit(2);
  }
}
