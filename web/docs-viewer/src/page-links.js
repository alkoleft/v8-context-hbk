export function parseGeneratedPageLink(href) {
  const value = String(href ?? "").trim();
  if (!value || value.startsWith("#") || value.startsWith("/") || /^[a-z][a-z0-9+.-]*:/i.test(value)) {
    return null;
  }
  const [pathPart, fragmentPart = ""] = value.split("#", 2);
  const fileName = pathPart.split("/").pop();
  if (!fileName?.endsWith(".md")) {
    return null;
  }
  const pageId = fileName.slice(0, -".md".length);
  if (!pageId.startsWith("page-") || !/^[\p{L}\p{N}_-]+$/u.test(pageId)) {
    return null;
  }
  return {
    pageId,
    fragment: fragmentPart || null,
  };
}

export function resolveGeneratedPageLink(href, currentPageId = null) {
  const generated = parseGeneratedPageLink(href);
  if (generated) {
    return { ...generated, samePage: generated.pageId === currentPageId };
  }

  const fragment = parseSamePageFragment(href);
  if (!fragment || !currentPageId) {
    return null;
  }
  return {
    pageId: currentPageId,
    fragment,
    samePage: true,
  };
}

function parseSamePageFragment(href) {
  const value = String(href ?? "").trim();
  if (!value.startsWith("#")) {
    return null;
  }
  const fragment = value.slice(1).split("?")[0].trim();
  return fragment || null;
}
