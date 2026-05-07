import { DocsDataClient } from "./data-client.js";
import { renderMarkdown } from "./markdown.js";
import { parseGeneratedPageLink } from "./page-links.js";

const client = new DocsDataClient();
const state = {
  manifest: null,
  locale: null,
  sectionCache: new Map(),
};

const localeBar = document.querySelector("#localeBar");
const tocRoot = document.querySelector("#tocRoot");
const pagePanel = document.querySelector("#pagePanel");

main().catch((error) => {
  showStatus(pagePanel, error.message);
});

pagePanel.addEventListener("click", (event) => {
  const link = event.target?.closest?.("a");
  if (!link) {
    return;
  }
  const target = parseGeneratedPageLink(link.getAttribute("href"));
  if (!target) {
    return;
  }
  event.preventDefault();
  openPageId(target.pageId, target.pageId, target.fragment).catch((error) => {
    showStatus(pagePanel, error.message);
  });
});

async function main() {
  state.manifest = await client.manifest();
  state.locale = state.manifest.locales[0];
  renderLocales();
  await loadLocaleRoot();
}

function renderLocales() {
  localeBar.replaceChildren(
    ...state.manifest.locales.map((locale) => {
      const button = document.createElement("button");
      button.className = "locale-button";
      button.type = "button";
      button.textContent = locale;
      button.setAttribute("aria-pressed", String(locale === state.locale));
      button.addEventListener("click", async () => {
        if (state.locale === locale) {
          return;
        }
        state.locale = locale;
        state.sectionCache.clear();
        renderLocales();
        await loadLocaleRoot();
      });
      return button;
    }),
  );
}

async function loadLocaleRoot() {
  showStatus(tocRoot, "Loading contents...");
  const rootPath = state.manifest.toc_roots[state.locale];
  const toc = await client.tocRoot(rootPath);
  tocRoot.replaceChildren(renderNodeGroup(toc.nodes));
}

function renderNodeGroup(nodes) {
  const group = document.createElement("div");
  group.className = "toc-group";
  for (const node of nodes) {
    group.append(renderNode(node));
  }
  return group;
}

function renderNode(node) {
  const wrapper = document.createElement("div");
  const row = document.createElement("div");
  row.className = "toc-row";

  const toggle = document.createElement("button");
  toggle.className = "toc-toggle";
  toggle.type = "button";
  toggle.textContent = node.has_children ? "+" : "";
  toggle.disabled = !node.has_children;
  toggle.setAttribute("aria-label", `Toggle ${node.title}`);
  toggle.setAttribute("aria-expanded", "false");

  const button = document.createElement("button");
  button.className = "toc-button";
  button.type = "button";
  button.textContent = node.title;
  button.addEventListener("click", () => {
    if (node.page_id) {
      openPageId(node.page_id, node.title, null);
    } else if (node.has_children) {
      toggle.click();
    }
  });

  row.append(toggle, button);
  wrapper.append(row);

  if (node.has_children) {
    const childContainer = document.createElement("div");
    childContainer.className = "toc-children";
    childContainer.hidden = true;
    wrapper.append(childContainer);
    toggle.addEventListener("click", async () => {
      const expanded = toggle.getAttribute("aria-expanded") === "true";
      if (expanded) {
        toggle.setAttribute("aria-expanded", "false");
        toggle.textContent = "+";
        childContainer.hidden = true;
        return;
      }
      toggle.setAttribute("aria-expanded", "true");
      toggle.textContent = "-";
      childContainer.hidden = false;
      if (!state.sectionCache.has(node.id)) {
        childContainer.textContent = "Loading...";
        const section = await client.tocSection(state.locale, node.children_path);
        state.sectionCache.set(node.id, section.nodes);
      }
      childContainer.replaceChildren(renderNodeGroup(state.sectionCache.get(node.id)));
    });
  }
  return wrapper;
}

async function openPageId(pageId, title, fragment) {
  showStatus(pagePanel, "Loading page...");
  const pageRoot = state.manifest.page_roots[state.locale];
  const markdown = await client.page(state.locale, pageRoot, pageId);
  pagePanel.replaceChildren(renderMarkdown(markdown));
  if (fragment) {
    document.getElementById(fragment)?.scrollIntoView();
  }
  document.title = `${title} - 1C Documentation`;
}

function showStatus(element, message) {
  const status = document.createElement("p");
  status.className = "status";
  status.textContent = message;
  element.replaceChildren(status);
}
