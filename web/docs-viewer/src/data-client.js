export class DocsDataClient {
  constructor(baseUrl = "/data") {
    this.baseUrl = baseUrl.replace(/\/+$/, "");
  }

  async manifest() {
    return this.#json("manifest.json");
  }

  async tocRoot(path) {
    return this.#json(path);
  }

  async tocSection(locale, path) {
    return this.#json(`locales/${encodePathSegment(locale)}/${path}`);
  }

  async page(locale, pageRoot, pageId) {
    return this.#text(`${pageRoot}/${encodePathSegment(pageId)}.md`);
  }

  async #json(path) {
    const response = await fetch(this.#url(path), { headers: { Accept: "application/json" } });
    if (!response.ok) {
      throw new Error(`Failed to load ${path}: HTTP ${response.status}`);
    }
    return response.json();
  }

  async #text(path) {
    const response = await fetch(this.#url(path), { headers: { Accept: "text/markdown,text/plain" } });
    if (!response.ok) {
      throw new Error(`Failed to load ${path}: HTTP ${response.status}`);
    }
    return response.text();
  }

  #url(path) {
    return `${this.baseUrl}/${String(path).replace(/^\/+/, "")}`;
  }
}

function encodePathSegment(value) {
  return encodeURIComponent(String(value));
}
