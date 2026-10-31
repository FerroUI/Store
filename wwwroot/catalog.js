// Shared by the browser, build, and tests. Manifests are untrusted input.
export const MAX_BYTES = 256 * 1024;
export const REFRESH_MS = 5 * 60 * 1000;
export const MAX_CACHE_AGE = 7 * 24 * 60 * 60 * 1000;
const string = (v, name, max = 4000) => {
  if (typeof v !== "string" || !v.trim() || v.length > max)
    throw new Error(`Invalid ${name}`);
  return v.trim();
};
const url = (value, base) => {
  const u = new URL(string(value, "URL", 2048), base);
  if (u.protocol !== "https:" || u.username || u.password)
    throw new Error("URLs must use HTTPS without credentials");
  return u.href;
};
export function validateManifest(raw, source, expectedId) {
  if (!raw || raw.schemaVersion !== 1)
    throw new Error("Unsupported manifest version");
  const id = string(raw.id, "id", 64);
  if (
    !/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(id) ||
    (expectedId && id !== expectedId)
  )
    throw new Error("Manifest identity mismatch");
  if (
    !raw.launch ||
    typeof raw.launch !== "object" ||
    Array.isArray(raw.launch)
  )
    throw new Error("Invalid launch");
  const launch = {
    url: url(raw.launch.url, source),
    mode: raw.launch.mode ?? "tab",
  };
  if (!["tab", "embed"].includes(launch.mode))
    throw new Error("Invalid launch mode");
  if (new URL(launch.url).origin !== new URL(source).origin)
    throw new Error("Launch origin must match manifest origin");
  const media = (list, video = false) => {
    if (list === undefined) return [];
    if (!Array.isArray(list) || list.length > 12)
      throw new Error("Invalid media list");
    return list.map((item) => ({
      url: url(item.url, source),
      alt: string(item.alt, "media caption", 240),
      ...(video
        ? {
            type: ["video/mp4", "video/webm"].includes(item.type)
              ? item.type
              : "video/mp4",
          }
        : {}),
    }));
  };
  const accent = raw.accent ?? "#76E4CF";
  if (!/^#[0-9a-f]{6}$/i.test(accent)) throw new Error("Invalid accent");
  const tags = raw.tags ?? [];
  if (!Array.isArray(tags) || tags.length > 12) throw new Error("Invalid tags");
  return {
    schemaVersion: 1,
    id,
    name: string(raw.name, "name", 80),
    summary: string(raw.summary, "summary", 180),
    description: string(raw.description, "description", 6000),
    category: string(raw.category, "category", 40),
    author: string(raw.author, "author", 80),
    icon: url(raw.icon, source),
    accent,
    launch,
    screenshots: media(raw.screenshots),
    videos: media(raw.videos, true),
    tags: tags.map((t) => string(t, "tag", 40)),
    ...(raw.version ? { version: string(raw.version, "version", 40) } : {}),
    ...(raw.source ? { source: url(raw.source, source) } : {}),
  };
}
export async function fetchManifest(
  entry,
  { fetcher = fetch, timeout = 6000 } = {},
) {
  const response = await fetcher(entry.manifest, {
    cache: "no-cache",
    signal: AbortSignal.timeout(timeout),
    credentials: "omit",
    referrerPolicy: "no-referrer",
  });
  if (!response.ok) throw new Error(`Manifest HTTP ${response.status}`);
  if (Number(response.headers.get("content-length")) > MAX_BYTES)
    throw new Error("Manifest too large");
  // Bound bytes while streaming, even when Content-Length is missing or compressed.
  const reader = response.body.getReader();
  let size = 0;
  const chunks = [];
  try {
    while (true) {
      const { value, done } = await reader.read();
      if (done) break;
      size += value.byteLength;
      if (size > MAX_BYTES) throw new Error("Manifest too large");
      chunks.push(value);
    }
  } finally {
    await reader.cancel().catch(() => {});
  }
  const bytes = new Uint8Array(size);
  let offset = 0;
  for (const c of chunks) {
    bytes.set(c, offset);
    offset += c.length;
  }
  return validateManifest(
    JSON.parse(new TextDecoder().decode(bytes)),
    entry.manifest,
    entry.id,
  );
}
function browserStorage() {
  try {
    return globalThis.localStorage;
  } catch {
    return null;
  }
}
export class LiveCatalog {
  constructor(
    bundle,
    { storage = browserStorage(), fetcher = fetch, now = Date.now } = {},
  ) {
    this.bundle = bundle;
    this.storage = storage;
    this.fetcher = fetcher;
    this.now = now;
    this.apps = new Map(bundle.apps.map((a) => [a.id, a]));
    this.status = new Map();
    this.checkedAt = 0;
    for (const entry of bundle.registry) {
      try {
        const cached = JSON.parse(
          storage?.getItem(`ferro-store:v1:${entry.id}`) ?? "null",
        );
        if (
          cached?.source === entry.manifest &&
          now() - cached.savedAt >= 0 &&
          now() - cached.savedAt < MAX_CACHE_AGE
        ) {
          this.apps.set(
            entry.id,
            validateManifest(cached.app, entry.manifest, entry.id),
          );
          this.status.set(entry.id, "cached");
        }
      } catch {
        /* Corrupt cache or unavailable storage: use the bundled listing. */
      }
    }
  }
  snapshot() {
    return this.bundle.registry.map((e) => this.apps.get(e.id)).filter(Boolean);
  }
  refresh(onChange = () => {}, { force = false } = {}) {
    if (this.pending) return this.pending;
    if (!force && this.checkedAt && this.now() - this.checkedAt < REFRESH_MS)
      return Promise.resolve(this.snapshot());
    this.checkedAt = this.now();
    this.pending = Promise.allSettled(
      this.bundle.registry.map(async (entry) => {
        try {
          const app = await fetchManifest(entry, { fetcher: this.fetcher });
          this.apps.set(entry.id, app);
          this.status.set(entry.id, "live");
          try {
            this.storage?.setItem(
              `ferro-store:v1:${entry.id}`,
              JSON.stringify({
                source: entry.manifest,
                savedAt: this.now(),
                app,
              }),
            );
          } catch {
            /* Browsing works without persistence. */
          }
          onChange(this.snapshot());
        } catch {
          this.status.set(entry.id, "fallback");
        }
      }),
    )
      .then(() => {
        onChange(this.snapshot());
        return this.snapshot();
      })
      .finally(() => {
        this.pending = null;
      });
    return this.pending;
  }
}
