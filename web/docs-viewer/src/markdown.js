export function renderMarkdown(markdown) {
  const root = document.createElement("div");
  root.className = "markdown";
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

  for (const rawLine of String(markdown).replace(/\r\n/g, "\n").split("\n")) {
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
  if (href.startsWith("#") || href.startsWith("/") || href.startsWith("./") || href.startsWith("../")) {
    return href;
  }
  if (/^https?:\/\//i.test(href)) {
    return href;
  }
  return "#";
}
