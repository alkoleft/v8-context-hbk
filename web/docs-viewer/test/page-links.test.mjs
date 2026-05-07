import assert from "node:assert/strict";
import test from "node:test";
import { parseGeneratedPageLink } from "../src/page-links.js";

test("parseGeneratedPageLink extracts generated page id and fragment", () => {
  assert.deepEqual(parseGeneratedPageLink("page-shlang-ru-select.md#HierarchicalWorkaround"), {
    pageId: "page-shlang-ru-select",
    fragment: "HierarchicalWorkaround",
  });
  assert.deepEqual(parseGeneratedPageLink("./page-страница-1.md"), {
    pageId: "page-страница-1",
    fragment: null,
  });
});

test("parseGeneratedPageLink ignores external and non-page links", () => {
  assert.equal(parseGeneratedPageLink("#local"), null);
  assert.equal(parseGeneratedPageLink("https://example.com/page.md"), null);
  assert.equal(parseGeneratedPageLink("/data/locales/ru/pages/page-a.md"), null);
  assert.equal(parseGeneratedPageLink("locales/ru/pages/a.md"), null);
  assert.equal(parseGeneratedPageLink("notes.txt#section"), null);
});
