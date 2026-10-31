# Ferro Store

A native FerroUI app store for GitHub Pages. Search and browse apps, open their detail pages, view screenshots and demo videos, and launch an app inside the store or in a new tab. The storefront uses Ferro controls, the WinUI light theme, Inter, and subtle composition Reveal background/border brushes. The approved Apple-inspired layout has editorial feature panels, real app screenshots, compact app rows, and mobile bottom navigation.

## Run it

Requirements: Node 24, Rust, and the Emscripten SDK. The framework is a pinned Git submodule; clone recursively:

```sh
git clone --recurse-submodules https://github.com/FerroUI/Store.git
cd Store
npm ci
vendor/FerroUI/scripts/browser/setup.sh ~/browser-tools
source ~/browser-tools/env.sh
npm run build
npm run preview
```

Open http://127.0.0.1:4173. Build output is in `dist/`, with relative asset paths so project-path hosting works. This uses a single-threaded WebAssembly build: no cross-origin isolation headers or service-worker reload are required, and embedded apps can manage their own isolation when needed. An embedded app may still require a separate tab if its own security headers prevent framing.

## How listings stay current

1. `catalog/apps.json` is the registry: stable app ID, public manifest URL, and starter manifest path.
2. A build fetches these manifests in parallel and writes `dist/catalog.json`. Until an app adopts the format (or if it is offline), the checked-in starter listing is used.
3. Visitors see that catalog or their most recent validated cache immediately.
4. The browser fetches each live manifest in parallel using `cache: no-cache` so HTTP caches revalidate with ETags/Last-Modified where the host supports them. Successful listings appear as they arrive; a slow host does not block other listings.
5. Manifests refresh every five minutes while visible, when returning to the page after that interval, or immediately with **Refresh**. Concurrent refreshes share one request per app. Each request times out after six seconds and is capped at 256 KiB.
6. Valid manifests are kept in localStorage for up to seven days. Failed, malformed, or incompatible updates keep the existing listing. Private browsing and disabled storage still work.

The runtime only reads URLs in the registry; apps cannot add other apps. A manifest cannot change its registered ID, use executable URL schemes, or launch on a different origin. Manifest strings are plain text. Remote icons/screenshots need CORS for conversion into Ferro bitmaps. HTML images and videos in the gallery can use ordinary public URLs.

For hosts other than the store's origin, serve manifests with `Access-Control-Allow-Origin: *` (or the store origin). GitHub Pages project sites under `ferroui.github.io` share an origin. There is no runtime proxy or backend. A completely unavailable store server cannot be visited offline; the listing cache is a resilience feature once the store has loaded.

## Publish an app manifest

Place **`ferro-app.json` beside your app's `index.html`**, include it in your Pages output, then add its URL to `catalog/apps.json`. The four initial manifests in `catalog/` are ready to copy into those apps' web roots. The starter screenshots come from the apps’ own repositories, pinned to exact revisions. Local copies in `wwwroot/media/` keep the initial collection fast; `sources.json` maps those immutable URLs to their bundled files. Live manifests can supply new media URLs normally.

See [`catalog/example.ferro-app.json`](catalog/example.ferro-app.json) for a full example and [`catalog/manifest.schema.json`](catalog/manifest.schema.json) for the v1 schema. Example:

```json
{
  "schemaVersion": 1,
  "id": "your-app",
  "name": "Your app",
  "summary": "A short reason to try it.",
  "description": "Describe what people can do with your app.",
  "category": "Productivity",
  "author": "Your name",
  "icon": "./icon.svg",
  "launch": {"url": "./", "mode": "embed"},
  "screenshots": [{"url": "./media/home.webp", "alt": "The main workspace"}],
  "videos": [{"url": "./media/demo.mp4", "alt": "A guided tour", "type": "video/mp4"}]
}
```

URLs resolve relative to the **manifest**, so moving an app between project paths works. Use public HTTPS URLs. `launch.mode` is `embed` or `tab` (default). The separate-tab action is always available. Optional fields: `accent` (six-digit hex), `tags`, `version`, `source`, `screenshots`, and `videos`. MP4/WebM support depends on the visitor's browser. Keep `id` stable. Unknown fields are ignored; unsupported `schemaVersion` values are rejected until the store supports them.

Each app has a shareable hash route, e.g. `#calculator`, which works on Pages without a server rewrite.

## Publish the store

The GitHub Actions workflow tests the metadata code, builds the native browser app, runs browser checks, uploads `dist/`, then deploys main to GitHub Pages. Pull requests build and test without deploying. Set **Settings → Pages → Source → GitHub Actions** once. The expected project URL is `https://ferroui.github.io/Store/`. The workflow uses the pinned framework revision including its Reveal light registry fix; update that submodule deliberately when upgrading Ferro.

## Checks

```sh
npm test                       # manifest validation, live refresh, caching, failure behavior
npx playwright install chromium
npx playwright test            # actual WebAssembly app, search, navigation, launches, live updates
```

The browser host (`wwwroot/main.js`) handles networking, SVG/raster conversion, media playback and iframe dialogs. The native interface and composition effects live in `src/main.rs`. `wwwroot/catalog.js` is shared by the build and runtime to keep validation identical.

MIT licensed. FerroUI is MIT licensed; see the pinned framework's notices for its dependencies.
