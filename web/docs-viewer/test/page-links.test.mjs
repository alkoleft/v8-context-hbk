import assert from "node:assert/strict";
import test from "node:test";
import { parseGeneratedPageLink, resolveGeneratedPageLink } from "../src/page-links.js";

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

test("resolveGeneratedPageLink resolves same-page fragments through the current page", () => {
  assert.deepEqual(resolveGeneratedPageLink("#Details", "page-ru-current"), {
    pageId: "page-ru-current",
    fragment: "Details",
    samePage: true,
  });
  assert.equal(resolveGeneratedPageLink("#Details", null), null);
});

test("resolveGeneratedPageLink distinguishes cross-page and current-page generated links", () => {
  assert.deepEqual(resolveGeneratedPageLink("page-ru-next.md#Details", "page-ru-current"), {
    pageId: "page-ru-next",
    fragment: "Details",
    samePage: false,
  });
  assert.deepEqual(resolveGeneratedPageLink("page-ru-current.md#Details", "page-ru-current"), {
    pageId: "page-ru-current",
    fragment: "Details",
    samePage: true,
  });
});
