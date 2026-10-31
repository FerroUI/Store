import { readFile, writeFile, mkdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { resolve, dirname } from "node:path";
import { fetchManifest, validateManifest } from "../wwwroot/catalog.js";
const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const registry = JSON.parse(
  await readFile(resolve(root, "catalog/apps.json"), "utf8"),
);
if (
  !Array.isArray(registry) ||
  new Set(registry.map((e) => e.id)).size !== registry.length
)
  throw new Error("Registry IDs must be unique");
const apps = await Promise.all(
  registry.map(async (entry) => {
    if (process.argv.includes("--offline")) return fallback(entry);
    try {
      return await fetchManifest(entry);
    } catch (e) {
      console.log(`${entry.id}: using starter listing (${e.message})`);
      return fallback(entry);
    }
  }),
);
async function fallback(entry) {
  if (!/^catalog\/[a-z0-9-]+\.json$/.test(entry.fallback))
    throw new Error("Invalid fallback path");
  return validateManifest(
    JSON.parse(await readFile(resolve(root, entry.fallback), "utf8")),
    entry.manifest,
    entry.id,
  );
}
await mkdir(resolve(root, "dist"), { recursive: true });
await writeFile(
  resolve(root, "dist/catalog.json"),
  JSON.stringify({ schemaVersion: 1, registry, apps }, null, 2) + "\n",
);
console.log(`Compiled ${apps.length} apps`);
