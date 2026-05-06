import assert from "node:assert/strict";
import test from "node:test";
import { DocsDataClient } from "../src/data-client.js";

test("DocsDataClient loads manifest, toc section and page through data endpoints", async () => {
  const requests = [];
  globalThis.fetch = async (url) => {
    requests.push(url);
    if (url.endsWith("manifest.json")) {
      return json({ locales: ["ru"] });
    }
    if (url.endsWith("toc-root.json")) {
      return json({ nodes: [] });
    }
    if (url.includes("toc-sections")) {
      return json({ parent_id: "node", nodes: [] });
    }
    if (url.endsWith(".md")) {
      return text("# Page");
    }
    return { ok: false, status: 404 };
  };

  const client = new DocsDataClient("/data/");
  assert.deepEqual(await client.manifest(), { locales: ["ru"] });
  assert.deepEqual(await client.tocRoot("locales/ru/toc-root.json"), { nodes: [] });
  assert.deepEqual(await client.tocSection("ru", "toc-sections/node.json"), {
    parent_id: "node",
    nodes: [],
  });
  assert.equal(await client.page("ru", "locales/ru/pages", "page-id"), "# Page");
  assert.deepEqual(requests, [
    "/data/manifest.json",
    "/data/locales/ru/toc-root.json",
    "/data/locales/ru/toc-sections/node.json",
    "/data/locales/ru/pages/page-id.md",
  ]);
});

function json(value) {
  return {
    ok: true,
    status: 200,
    async json() {
      return value;
    },
  };
}

function text(value) {
  return {
    ok: true,
    status: 200,
    async text() {
      return value;
    },
  };
}
