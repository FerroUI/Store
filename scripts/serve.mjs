import http from "node:http";
import { readFile } from "node:fs/promises";
import { resolve, extname, sep } from "node:path";
const root = resolve("dist");
const types = {
  ".html": "text/html",
  ".js": "text/javascript",
  ".json": "application/json",
  ".wasm": "application/wasm",
  ".css": "text/css",
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".webp": "image/webp",
};
http
  .createServer(async (req, res) => {
    try {
      let pathname = decodeURIComponent(
        new URL(req.url, "http://localhost").pathname,
      );
      // Mirror GitHub Pages project-path hosting during local checks.
      if (pathname.startsWith("/FerroStore/"))
        pathname = pathname.slice("/FerroStore".length);
      const path = resolve(root, "." + pathname);
      if (path !== root && !path.startsWith(root + sep))
        throw new Error("Forbidden");
      const data = await readFile(
        path === root ? resolve(root, "index.html") : path,
      );
      res
        .writeHead(200, {
          "Content-Type":
            types[extname(path)] ??
            (path === root ? "text/html" : "application/octet-stream"),
          "Cache-Control": "no-cache",
        })
        .end(data);
    } catch {
      res.writeHead(404).end("Not found");
    }
  })
  .listen(4173, "127.0.0.1", () => console.log("Local: http://127.0.0.1:4173"));
