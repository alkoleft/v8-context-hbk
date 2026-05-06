import assert from "node:assert/strict";
import test from "node:test";
import { renderMarkdown } from "../src/markdown.js";

test("renderMarkdown displays html-like input as text", () => {
  const document = new TestDocument();
  globalThis.document = document;
  const rendered = renderMarkdown("# Title\n\n<script>alert(1)</script>");
  assert.equal(rendered.children[0].tagName, "h1");
  assert.equal(rendered.children[0].textContent, "Title");
  assert.equal(rendered.children[1].tagName, "p");
  assert.equal(rendered.children[1].textContent, "<script>alert(1)</script>");
  assert.equal(rendered.children[1].innerHTML, "");
});

test("renderMarkdown handles basic inline formatting without raw html", () => {
  const document = new TestDocument();
  globalThis.document = document;
  const rendered = renderMarkdown("Open **Object** and [page](locales/ru/pages/a.md)");
  const paragraph = rendered.children[0];
  const inlineElements = paragraph.children.filter((child) => typeof child !== "string");
  assert.equal(paragraph.textContent, "Open Object and page");
  assert.equal(inlineElements[0].tagName, "strong");
  assert.equal(inlineElements[0].textContent, "Object");
  assert.equal(inlineElements[1].tagName, "a");
  assert.equal(inlineElements[1].attributes.get("href"), "#");
});

class TestDocument {
  createElement(tagName) {
    return new TestElement(tagName);
  }
}

class TestElement {
  constructor(tagName) {
    this.tagName = tagName;
    this.children = [];
    this.attributes = new Map();
    this.hidden = false;
    this.innerHTML = "";
    this.textContent = "";
  }

  append(...children) {
    this.children.push(...children);
    this.textContent += children
      .map((child) => (typeof child === "string" ? child : child.textContent))
      .join("");
  }

  set className(value) {
    this.attributes.set("class", value);
  }

  get className() {
    return this.attributes.get("class") ?? "";
  }

  set href(value) {
    this.attributes.set("href", value);
  }

  get href() {
    return this.attributes.get("href") ?? "";
  }
}
