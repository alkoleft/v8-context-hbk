import assert from "node:assert/strict";
import test from "node:test";
import { pageTitleFromRenderedMarkdown } from "../src/navigation.js";

test("pageTitleFromRenderedMarkdown uses the first rendered heading", () => {
  const rendered = {
    children: [
      { tagName: "p", textContent: "intro" },
      { tagName: "h1", textContent: "Human Page Title" },
    ],
  };

  assert.equal(pageTitleFromRenderedMarkdown(rendered, "page-ru-opaque"), "Human Page Title");
});

test("pageTitleFromRenderedMarkdown falls back to a human TOC title", () => {
  const rendered = {
    children: [{ tagName: "p", textContent: "body" }],
  };

  assert.equal(pageTitleFromRenderedMarkdown(rendered, "TOC Title"), "TOC Title");
});
