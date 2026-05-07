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

test("renderMarkdown turns generated heading anchors into invisible anchors", () => {
  const document = new TestDocument();
  globalThis.document = document;
  const rendered = renderMarkdown('<a id="HierarchicalWorkaround"></a>\n## Section');
  assert.equal(rendered.children[0].tagName, "a");
  assert.equal(rendered.children[0].attributes.get("id"), "HierarchicalWorkaround");
  assert.equal(rendered.children[0].attributes.get("aria-hidden"), "true");
  assert.equal(rendered.children[0].textContent, "");
  assert.equal(rendered.children[1].tagName, "h2");
  assert.equal(rendered.children[1].textContent, "Section");
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

test("renderMarkdown preserves generated page links for app routing", () => {
  const document = new TestDocument();
  globalThis.document = document;
  const rendered = renderMarkdown("[Next](page-ru-314485b4b83f6ad6.md#Details)");
  const link = rendered.children[0].children[0];
  assert.equal(link.tagName, "a");
  assert.equal(link.attributes.get("href"), "page-ru-314485b4b83f6ad6.md#Details");
});

test("renderMarkdown renders blockquotes without raw markdown markers", () => {
  const document = new TestDocument();
  globalThis.document = document;
  const rendered = renderMarkdown("> Программа запуска - 1CEStart\n>\n> Клиентское приложение");
  const blockquote = rendered.children[0];
  assert.equal(blockquote.tagName, "blockquote");
  assert.equal(blockquote.children[0].tagName, "p");
  assert.equal(blockquote.children[0].textContent, "Программа запуска - 1CEStart");
  assert.equal(blockquote.children[1].tagName, "p");
  assert.equal(blockquote.children[1].textContent, "Клиентское приложение");
  assert.equal(blockquote.textContent.includes(">"), false);
});

test("renderMarkdown renders GFM tables as DOM tables", () => {
  const document = new TestDocument();
  globalThis.document = document;
  const rendered = renderMarkdown("| Имя | Значение |\n| --- | --- |\n| ВЫБОР | CASE |");
  const table = rendered.children[0];
  assert.equal(table.tagName, "table");
  assert.equal(table.children[0].tagName, "thead");
  assert.equal(table.children[0].children[0].children[0].tagName, "th");
  assert.equal(table.children[0].children[0].children[0].textContent, "Имя");
  assert.equal(table.children[1].tagName, "tbody");
  assert.equal(table.children[1].children[0].children[1].tagName, "td");
  assert.equal(table.children[1].children[0].children[1].textContent, "CASE");
});

test("renderMarkdown renders quoted GFM tables inside blockquotes", () => {
  const document = new TestDocument();
  globalThis.document = document;
  const rendered = renderMarkdown("> | Программа запуска | |\n> | --- | --- |\n> | Клиентское приложение | |");
  const blockquote = rendered.children[0];
  const table = blockquote.children[0];
  assert.equal(blockquote.tagName, "blockquote");
  assert.equal(table.tagName, "table");
  assert.equal(table.children[0].children[0].children[0].textContent, "Программа запуска");
  assert.equal(table.children[1].children[0].children[0].textContent, "Клиентское приложение");
  assert.equal(blockquote.textContent.includes("> |"), false);
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

  setAttribute(name, value) {
    this.attributes.set(name, String(value));
  }
}
