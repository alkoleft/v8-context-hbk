import assert from "node:assert/strict";
import { chmod, mkdtemp, mkdir, symlink, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { createDocsViewerServer, parseCliArgs, resolveWithin } from "../src/server.mjs";

test("parseCliArgs requires data root and host:port listen address", () => {
  assert.throws(() => parseCliArgs([]), /missing required --data/);
  assert.throws(
    () => parseCliArgs(["--data", "data", "--listen", "127.0.0.1"]),
    /invalid --listen/,
  );
  assert.deepEqual(parseCliArgs(["--data", "data", "--listen", "127.0.0.1:4173"]), {
    dataRoot: "data",
    host: "127.0.0.1",
    port: 4173,
  });
});

test("resolveWithin rejects traversal outside root", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "docs-viewer-root-"));
  assert.equal(resolveWithin(root, "manifest.json"), path.join(root, "manifest.json"));
  assert.throws(() => resolveWithin(root, "../secret.txt"), /Forbidden/);
  assert.throws(() => resolveWithin(root, "%2e%2e/secret.txt"), /Forbidden/);
});

test("server serves static and data files while confining data paths", async () => {
  const temp = await mkdtemp(path.join(os.tmpdir(), "docs-viewer-server-"));
  const dataRoot = path.join(temp, "data");
  const staticRoot = path.join(temp, "static");
  await mkdir(dataRoot);
  await mkdir(staticRoot);
  await writeFile(path.join(dataRoot, "manifest.json"), "{\"locales\":[\"ru\"]}");
  await writeFile(path.join(temp, "secret.txt"), "outside");
  const outsideDir = path.join(temp, "outside");
  await mkdir(outsideDir);
  await writeFile(path.join(outsideDir, "nested.md"), "nested outside");
  await symlink(path.join(temp, "secret.txt"), path.join(dataRoot, "leak.md"));
  await symlink(outsideDir, path.join(dataRoot, "linked"));
  await writeFile(path.join(staticRoot, "index.html"), "<!doctype html>");

  const server = await createDocsViewerServer({ dataRoot, staticRoot });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  const { port } = server.address();
  try {
    const base = `http://127.0.0.1:${port}`;
    assert.equal((await fetch(`${base}/`)).status, 200);
    assert.deepEqual(await (await fetch(`${base}/data/manifest.json`)).json(), {
      locales: ["ru"],
    });
    assert.equal((await fetch(`${base}/data/missing.json`)).status, 404);
    assert.equal((await fetch(`${base}/data/%2e%2e%2fsecret.txt`)).status, 403);
    assert.equal((await fetch(`${base}/data/leak.md`)).status, 403);
    assert.equal((await fetch(`${base}/data/linked/nested.md`)).status, 403);
  } finally {
    await new Promise((resolve) => server.close(resolve));
  }
});

test("server rejects unreadable data files before sending success headers", async () => {
  const temp = await mkdtemp(path.join(os.tmpdir(), "docs-viewer-unreadable-"));
  const dataRoot = path.join(temp, "data");
  const staticRoot = path.join(temp, "static");
  await mkdir(dataRoot);
  await mkdir(staticRoot);
  await writeFile(path.join(staticRoot, "index.html"), "<!doctype html>");
  const unreadable = path.join(dataRoot, "secret.md");
  await writeFile(unreadable, "secret");
  await chmod(unreadable, 0o000);

  const server = await createDocsViewerServer({ dataRoot, staticRoot });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  const { port } = server.address();
  try {
    assert.equal((await fetch(`http://127.0.0.1:${port}/data/secret.md`)).status, 403);
  } finally {
    await chmod(unreadable, 0o600);
    await new Promise((resolve) => server.close(resolve));
  }
});

test("createDocsViewerServer rejects invalid data root", async () => {
  await assert.rejects(
    () => createDocsViewerServer({ dataRoot: "missing", staticRoot: "." }),
    /--data 'missing' is not a directory/,
  );
});
