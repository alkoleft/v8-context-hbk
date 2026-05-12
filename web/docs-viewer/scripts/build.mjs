import { copyFile, mkdir, rm } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const dist = path.join(root, "dist");

await rm(dist, { recursive: true, force: true });
await mkdir(dist, { recursive: true });

for (const [from, to] of [
  ["public/index.html", "index.html"],
  ["public/styles.css", "styles.css"],
  ["src/app.js", "app.js"],
  ["src/data-client.js", "data-client.js"],
  ["src/markdown.js", "markdown.js"],
  ["src/navigation.js", "navigation.js"],
  ["src/page-links.js", "page-links.js"],
]) {
  await copyFile(path.join(root, from), path.join(dist, to));
}

console.log(`built docs viewer into ${dist}`);
