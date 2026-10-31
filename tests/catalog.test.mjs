import { test } from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import {
  LiveCatalog,
  validateManifest,
  fetchManifest,
  MAX_BYTES,
  MAX_CACHE_AGE,
} from "../wwwroot/catalog.js";
const raw = JSON.parse(
  await readFile(new URL("../catalog/calculator.json", import.meta.url)),
);
const entry = {
  id: "calculator",
  manifest: "https://ferroui.github.io/calculator/ferro-app.json",
};
const app = validateManifest(raw, entry.manifest, entry.id);
const bundle = { registry: [entry], apps: [app] };
const storage = () => {
  const map = new Map();
  return { getItem: (k) => map.get(k), setItem: (k, v) => map.set(k, v) };
};
const ok = (value) => async () =>
  new Response(JSON.stringify(value), {
    headers: { "content-type": "application/json" },
  });
test("resolves app-relative URLs beneath the app project path", () => {
  assert.equal(app.icon, "https://ferroui.github.io/calculator/favicon.svg");
  assert.equal(app.launch.url, "https://ferroui.github.io/calculator/");
});
test("rejects unsupported versions, identity swaps, unsafe URLs, other origins, and invalid launch modes", () => {
  for (const patch of [
    { launch: null },
    { launch: [] },
    { launch: {} },
    { schemaVersion: 2 },
    { id: "other" },
    { icon: "javascript:alert(1)" },
    { launch: { url: "https://evil.example/app" } },
    { launch: { url: "./", mode: "popup" } },
    { accent: "red" },
    { tags: "invalid" },
  ])
    assert.throws(() =>
      validateManifest({ ...raw, ...patch }, entry.manifest, entry.id),
    );
});
test("supports screenshots and demo videos with app-relative URLs", () => {
  const a = validateManifest(
    {
      ...raw,
      screenshots: [{ url: "screens/one.png", alt: "Standard mode" }],
      videos: [{ url: "demo.webm", alt: "A quick tour", type: "video/webm" }],
    },
    entry.manifest,
  );
  assert.equal(
    a.screenshots[0].url,
    "https://ferroui.github.io/calculator/screens/one.png",
  );
  assert.equal(a.videos[0].type, "video/webm");
});
test("refresh updates immediately and persists the live manifest", async () => {
  const saved = storage();
  const live = new LiveCatalog(bundle, {
    storage: saved,
    fetcher: ok({ ...raw, name: "New name" }),
  });
  assert.equal(live.snapshot()[0].name, app.name);
  let changes = 0;
  await live.refresh(() => changes++);
  assert.equal(live.snapshot()[0].name, "New name");
  assert.ok(changes > 0);
  assert.equal(
    new LiveCatalog(bundle, { storage: saved }).snapshot()[0].name,
    "New name",
  );
});
test("404, offline, timeout, and invalid manifests retain the existing listing", async () => {
  for (const fetcher of [
    async () => new Response("", { status: 404 }),
    async () => {
      throw new TypeError("offline");
    },
    async () => {
      throw new DOMException("timeout", "TimeoutError");
    },
    ok({ ...raw, id: "evil" }),
  ]) {
    const live = new LiveCatalog(bundle, { storage: storage(), fetcher });
    await live.refresh();
    assert.deepEqual(live.snapshot(), [app]);
    assert.equal(live.status.get(entry.id), "fallback");
  }
});
test("independent fetches update fast apps while slower apps are pending", async () => {
  let finish;
  const slow = new Promise((r) => (finish = r));
  const e2 = {
    id: "ember",
    manifest: "https://ferroui.github.io/Ember/ferro-app.json",
  };
  const emberRaw = JSON.parse(
    await readFile(new URL("../catalog/ember.json", import.meta.url)),
  );
  const live = new LiveCatalog(
    {
      registry: [entry, e2],
      apps: [app, validateManifest(emberRaw, e2.manifest)],
    },
    {
      storage: storage(),
      fetcher: async (u) =>
        u === entry.manifest
          ? new Response(JSON.stringify({ ...raw, name: "Live first" }))
          : slow,
    },
  );
  const pending = live.refresh();
  await new Promise((r) => setTimeout(r, 15));
  assert.equal(live.snapshot()[0].name, "Live first");
  finish(new Response(JSON.stringify(emberRaw)));
  await pending;
});
test("deduplicates concurrent refreshes and respects the refresh interval", async () => {
  let count = 0;
  const live = new LiveCatalog(bundle, {
    storage: storage(),
    now: () => 1000000,
    fetcher: async () => {
      count++;
      return new Response(JSON.stringify(raw));
    },
  });
  await Promise.all([live.refresh(), live.refresh()]);
  await live.refresh();
  assert.equal(count, 1);
  await live.refresh(() => {}, { force: true });
  assert.equal(count, 2);
});
test("expired, corrupt, poisoned, and registry-mismatched caches are ignored", () => {
  for (const cached of [
    "bad json",
    JSON.stringify({ source: entry.manifest, savedAt: 0, app }),
    JSON.stringify({
      source: "https://old.example/",
      savedAt: MAX_CACHE_AGE,
      app: { ...app, name: "poison" },
    }),
    JSON.stringify({
      source: entry.manifest,
      savedAt: MAX_CACHE_AGE,
      app: { ...app, icon: "data:bad" },
    }),
  ]) {
    const s = { getItem: () => cached };
    assert.deepEqual(
      new LiveCatalog(bundle, {
        storage: s,
        now: () => MAX_CACHE_AGE + 1,
      }).snapshot(),
      [app],
    );
  }
});
test("browsing and refresh work when browser storage throws", async () => {
  const live = new LiveCatalog(bundle, {
    storage: {
      getItem() {
        throw Error();
      },
      setItem() {
        throw Error();
      },
    },
    fetcher: ok({ ...raw, name: "Fresh" }),
  });
  await live.refresh();
  assert.equal(live.snapshot()[0].name, "Fresh");
});
test("fetch revalidates HTTP caches and sets a timeout", async () => {
  await fetchManifest(entry, {
    fetcher: async (u, options) => {
      assert.equal(u, entry.manifest);
      assert.equal(options.cache, "no-cache");
      assert.equal(options.credentials, "omit");
      assert.ok(options.signal);
      return new Response(JSON.stringify(raw));
    },
  });
});
test("rejects oversized streaming bodies without trusting Content-Length", async () => {
  await assert.rejects(
    fetchManifest(entry, {
      fetcher: async () => new Response(" ".repeat(MAX_BYTES + 1)),
    }),
    /too large/,
  );
});
test("all four starter manifests conform to the shared contract", async () => {
  const list = JSON.parse(
    await readFile(new URL("../catalog/apps.json", import.meta.url)),
  );
  assert.equal(list.length, 4);
  for (const e of list)
    assert.equal(
      validateManifest(
        JSON.parse(
          await readFile(new URL("../" + e.fallback, import.meta.url)),
        ),
        e.manifest,
        e.id,
      ).id,
      e.id,
    );
});
