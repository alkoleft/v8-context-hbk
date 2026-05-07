import { parseGeneratedPageLink } from "./page-links.js";

export function renderMarkdown(markdown) {
  const root = document.createElement("div");
  root.className = "markdown";
  const lines = String(markdown).replace(/\r\n/g, "\n").split("\n");
  let paragraph = [];
  let list = null;
  let code = null;

  const flushParagraph = () => {
    if (paragraph.length === 0) {
      return;
    }
    const p = document.createElement("p");
    appendInline(p, paragraph.join(" "));
    root.append(p);
    paragraph = [];
  };

  const flushList = () => {
    if (list) {
      root.append(list);
      list = null;
    }
  };

  for (let lineIndex = 0; lineIndex < lines.length; lineIndex += 1) {
    const rawLine = lines[lineIndex];
    const line = rawLine.trimEnd();
    if (line.startsWith("```")) {
      flushParagraph();
      flushList();
      if (code) {
        root.append(code.pre);
        code = null;
      } else {
        const pre = document.createElement("pre");
        const codeElement = document.createElement("code");
        pre.append(codeElement);
        code = { pre, codeElement, lines: [] };
      }
      continue;
    }
    if (code) {
      code.lines.push(rawLine);
      code.codeElement.textContent = code.lines.join("\n");
      continue;
    }
    if (!line.trim()) {
      flushParagraph();
      flushList();
      continue;
    }
    const generatedAnchor = line.match(/^<a\s+(?:id|name)="([^"]+)"><\/a>$/i);
    if (generatedAnchor) {
      flushParagraph();
      flushList();
      const anchor = document.createElement("a");
      anchor.setAttribute("id", generatedAnchor[1]);
      anchor.setAttribute("aria-hidden", "true");
      root.append(anchor);
      continue;
    }
    const quoteLine = line.match(/^>\s?(.*)$/);
    if (quoteLine) {
      flushParagraph();
      flushList();
      const quoteLines = [quoteLine[1]];
      while (lineIndex + 1 < lines.length) {
        const nextQuoteLine = lines[lineIndex + 1].trimEnd().match(/^>\s?(.*)$/);
        if (!nextQuoteLine) {
          break;
        }
        quoteLines.push(nextQuoteLine[1]);
        lineIndex += 1;
      }
      const blockquote = document.createElement("blockquote");
      appendRenderedChildren(blockquote, renderMarkdown(quoteLines.join("\n")));
      root.append(blockquote);
      continue;
    }
    if (isTableHeader(lines, lineIndex)) {
      flushParagraph();
      flushList();
      const table = document.createElement("table");
      const thead = document.createElement("thead");
      const tbody = document.createElement("tbody");
      appendTableRow(thead, "th", parseTableRow(lines[lineIndex]));
      lineIndex += 1;
      while (lineIndex + 1 < lines.length && isTableRow(lines[lineIndex + 1])) {
        lineIndex += 1;
        appendTableRow(tbody, "td", parseTableRow(lines[lineIndex]));
      }
      table.append(thead, tbody);
      root.append(table);
      continue;
    }
    const heading = line.match(/^(#{1,3})\s+(.+)$/);
    if (heading) {
      flushParagraph();
      flushList();
      const level = heading[1].length;
      const element = document.createElement(`h${level}`);
      appendInline(element, heading[2].trim());
      root.append(element);
      continue;
    }
    const listItem = line.match(/^[-*]\s+(.+)$/);
    if (listItem) {
      flushParagraph();
      if (!list) {
        list = document.createElement("ul");
      }
      const item = document.createElement("li");
      appendInline(item, listItem[1].trim());
      list.append(item);
      continue;
    }
    paragraph.push(line.trim());
  }
  if (code) {
    root.append(code.pre);
  }
  flushParagraph();
  flushList();
  return root;
}

function appendRenderedChildren(parent, rendered) {
  parent.append(...Array.from(rendered.children));
}

function isTableHeader(lines, lineIndex) {
  return (
    lineIndex + 1 < lines.length &&
    isTableRow(lines[lineIndex]) &&
    isTableSeparator(lines[lineIndex + 1])
  );
}

function isTableRow(line) {
  const cells = parseTableRow(line);
  return cells.length > 1;
}

function isTableSeparator(line) {
  const cells = parseTableRow(line);
  return cells.length > 1 && cells.every((cell) => /^:?-{3,}:?$/.test(cell));
}

function parseTableRow(line) {
  const trimmed = String(line).trim();
  if (!trimmed.includes("|")) {
    return [];
  }
  const withoutEdges = trimmed.replace(/^\|/, "").replace(/\|$/, "");
  return withoutEdges.split("|").map((cell) => cell.trim());
}

function appendTableRow(parent, cellTagName, cells) {
  const row = document.createElement("tr");
  for (const cellText of cells) {
    const cell = document.createElement(cellTagName);
    appendInline(cell, cellText);
    row.append(cell);
  }
  parent.append(row);
}

function appendInline(parent, text) {
  const pattern = /(\*\*([^*]+)\*\*|\[([^\]]+)\]\(([^)]+)\))/g;
  let index = 0;
  for (const match of text.matchAll(pattern)) {
    appendText(parent, text.slice(index, match.index));
    if (match[2]) {
      const strong = document.createElement("strong");
      strong.textContent = match[2];
      parent.append(strong);
    } else {
      const anchor = document.createElement("a");
      anchor.textContent = match[3];
      anchor.href = safeHref(match[4]);
      parent.append(anchor);
    }
    index = match.index + match[0].length;
  }
  appendText(parent, text.slice(index));
}

function appendText(parent, text) {
  if (text) {
    parent.append(text);
  }
}

function safeHref(value) {
  const href = String(value).trim();
  if (parseGeneratedPageLink(href)) {
    return href;
  }
  if (href.startsWith("#") || href.startsWith("/") || href.startsWith("./") || href.startsWith("../")) {
    return href;
  }
  if (/^https?:\/\//i.test(href)) {
    return href;
  }
  return "#";
}
