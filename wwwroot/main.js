import { FerroExports } from "./ferroui.js";
import { loadModule } from "./ferroui-loader.js";
import { LiveCatalog } from "./catalog.js";
let runtime,
  catalog,
  bundledAssets = {};
const dialog = document.querySelector("#app-dialog");
const content = document.querySelector("#dialog-content");
const title = document.querySelector("#dialog-title");
const description = document.querySelector("#dialog-description");
const external = document.querySelector("#external-link");
const close = () => {
  dialog.close();
  content.replaceChildren();
};
document.querySelector("#close-dialog").addEventListener("click", close);
dialog.addEventListener("cancel", () => content.replaceChildren());
dialog.addEventListener("click", (event) => {
  if (event.target === dialog) close();
});
function openDialog(app) {
  title.textContent = app.name;
  description.textContent =
    "If the app does not load here, open it in a new tab.";
  external.href = app.launch.url;
  content.replaceChildren();
  if (!dialog.open) dialog.showModal();
}
function route() {
  let id = "";
  try {
    id = decodeURIComponent(location.hash.replace(/^#\/?/, ""));
  } catch {
    /* Invalid deep link goes home. */
  }
  runtime?.navigate(catalog?.snapshot().some((app) => app.id === id) ? id : "");
}
function navigate(id) {
  if (location.hash.slice(1) === id) route();
  else location.hash = id;
}
const assets = new Map();
async function loadImage(url) {
  const cached = assets.get(url);
  if (cached && Date.now() - cached.at < 5 * 60 * 1000) return cached.task;
  const task = (async () => {
    // Decode via the browser to support SVG app icons as well as screenshots.
    const source = bundledAssets[url]
      ? new URL(bundledAssets[url], import.meta.url).href
      : url;
    const response = await fetch(source, {
      signal: AbortSignal.timeout(8000),
      credentials: "omit",
    });
    if (
      !response.ok ||
      Number(response.headers.get("content-length")) > 12_000_000
    )
      return;
    const blob = await response.blob();
    if (blob.size > 12_000_000) return;
    const object = URL.createObjectURL(blob);
    try {
      const img = new Image();
      img.src = object;
      await img.decode();
      const scale = Math.min(
        1,
        1400 / Math.max(img.naturalWidth, img.naturalHeight),
      );
      const canvas = document.createElement("canvas");
      canvas.width = Math.max(1, Math.round(img.naturalWidth * scale));
      canvas.height = Math.max(1, Math.round(img.naturalHeight * scale));
      canvas.getContext("2d").drawImage(img, 0, 0, canvas.width, canvas.height);
      const png = await new Promise((resolve) =>
        canvas.toBlob(resolve, "image/png"),
      );
      if (png) runtime.setImage(url, new Uint8Array(await png.arrayBuffer()));
    } finally {
      URL.revokeObjectURL(object);
    }
  })().catch(() => {});
  assets.set(url, { task, at: Date.now() });
  return task;
}
globalThis.ferroStoreImagesReady = () =>
  Promise.all([...assets.values()].map((asset) => asset.task));
function images() {
  for (const app of catalog.snapshot()) {
    loadImage(app.icon);
    if (app.screenshots[0]) loadImage(app.screenshots[0].url);
  }
}
async function refresh(force = false) {
  runtime?.setStatus("Refreshing the collection…");
  await catalog.refresh(
    (apps) => {
      runtime?.updateCatalog(JSON.stringify(apps));
      images();
    },
    { force },
  );
  const live = [...catalog.status.values()].filter((v) => v === "live").length;
  runtime?.setStatus(
    live
      ? "Collection refreshed just now"
      : `${catalog.snapshot().length} apps · Ready to explore`,
  );
}
globalThis.ferroStoreAction = (action, payload) => {
  if (action === "navigate") return navigate(payload);
  if (action === "search") {
    if (location.hash)
      history.replaceState(null, "", `${location.pathname}${location.search}`);
    return;
  }
  if (action === "refresh") return refresh(true);
  if (action === "framework")
    return window.open(
      "https://github.com/wieslawsoltes/FerroUI",
      "_blank",
      "noopener,noreferrer",
    );
  if (action === "source")
    return (
      /^https:\/\//.test(payload) &&
      window.open(payload, "_blank", "noopener,noreferrer")
    );
  let app;
  try {
    const id = JSON.parse(payload).id;
    app = catalog.snapshot().find((a) => a.id === id);
  } catch {
    return;
  }
  if (!app) return;
  if (action === "tab" || (action === "launch" && app.launch.mode === "tab"))
    return window.open(app.launch.url, "_blank", "noopener,noreferrer");
  if (action === "launch") {
    openDialog(app);
    const frame = document.createElement("iframe");
    frame.title = app.name;
    frame.src = app.launch.url;
    frame.setAttribute(
      "sandbox",
      "allow-scripts allow-same-origin allow-forms allow-downloads allow-popups allow-modals",
    );
    frame.allow = "clipboard-read; clipboard-write; fullscreen";
    content.append(frame);
  }
  if (action === "media") {
    openDialog(app);
    description.textContent = "Screenshots & demo videos";
    for (const [items, isVideo] of [
      [app.screenshots, false],
      [app.videos, true],
    ])
      for (const item of items) {
        const figure = document.createElement("figure");
        figure.className = "media-item";
        const media = document.createElement(isVideo ? "video" : "img");
        media.src = bundledAssets[item.url]
          ? new URL(bundledAssets[item.url], import.meta.url).href
          : item.url;
        if (isVideo) {
          media.controls = true;
          media.preload = "metadata";
        } else {
          media.alt = item.alt;
          media.loading = "lazy";
        }
        const caption = document.createElement("figcaption");
        caption.textContent = item.alt;
        figure.append(media, caption);
        content.append(figure);
      }
  }
};
try {
  const [bundle, sourceAssets, loaded] = await Promise.all([
    fetch("./catalog.json").then((r) => {
      if (!r.ok) throw new Error("Catalog unavailable");
      return r.json();
    }),
    fetch("./media/sources.json")
      .then((r) => (r.ok ? r.json() : {}))
      .catch(() => ({})),
    loadModule({
      script: "./ferro-store.js",
      element: document.querySelector("#out"),
    }),
  ]);
  let storage;
  try {
    storage = localStorage;
  } catch {
    storage = null;
  }
  bundledAssets = sourceAssets;
  catalog = new LiveCatalog(bundle, { storage });
  if (!loaded) throw new Error("The Ferro runtime could not start");
  runtime = await loaded.createRuntime();
  FerroExports.attach(runtime);
  runtime.runMain(JSON.stringify(catalog.snapshot()), innerWidth);
  globalThis.ferroStore = runtime;
  document.querySelector("#splash").remove();
  images();
  route();
  window.addEventListener("hashchange", route);
  let resizeFrame;
  window.addEventListener("resize", () => {
    cancelAnimationFrame(resizeFrame);
    resizeFrame = requestAnimationFrame(() => runtime.resize(innerWidth));
  });
  refresh();
  document.addEventListener("visibilitychange", () => {
    if (document.visibilityState === "visible") refresh();
  });
  setInterval(
    () => {
      if (document.visibilityState === "visible") refresh();
    },
    5 * 60 * 1000,
  );
} catch (error) {
  console.error(error);
  let splash = document.querySelector("#splash");
  if (!splash) {
    splash = document.createElement("section");
    splash.id = "splash";
    document.body.append(splash);
  }
  let message = document.querySelector("#loading-message");
  if (!message) {
    message = document.createElement("p");
    message.id = "loading-message";
    splash.append(message);
  }
  message.textContent = "The store could not open. Reload to try again.";
  const links = document.createElement("p");
  for (const [name, path] of [
    ["FerroDesign", "FerroDesign"],
    ["Calculator", "calculator"],
    ["Ember", "Ember"],
    ["Solitaire", "Solitaire"],
  ]) {
    const a = document.createElement("a");
    a.textContent = `${name} ↗  `;
    a.href = `https://ferroui.github.io/${path}/`;
    links.append(a);
  }
  document.querySelector("#splash")?.append(links);
}
