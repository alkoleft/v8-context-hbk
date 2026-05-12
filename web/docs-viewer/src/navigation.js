export function pageTitleFromRenderedMarkdown(rendered, fallback) {
  const title = firstHeadingText(rendered)?.trim();
  if (title) {
    return title;
  }
  return String(fallback ?? "").trim() || "Documentation";
}

function firstHeadingText(root) {
  for (const child of Array.from(root?.children ?? [])) {
    const tagName = String(child?.tagName ?? "").toLowerCase();
    if (tagName === "h1" || tagName === "h2" || tagName === "h3") {
      return child.textContent;
    }
  }
  return null;
}
