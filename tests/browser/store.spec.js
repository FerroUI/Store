import { test, expect } from "@playwright/test";
import { readFile } from "node:fs/promises";
const sample = JSON.parse(
  await readFile(new URL("../../catalog/calculator.json", import.meta.url)),
);
const state = (page) =>
  page.evaluate(() => JSON.parse(globalThis.ferroStore.uiState()));
async function frame(page) {
  await page.evaluate(
    () =>
      new Promise((resolve) =>
        requestAnimationFrame(() => requestAnimationFrame(resolve)),
      ),
  );
}
async function button(page, text) {
  await frame(page);
  let b = (await state(page)).buttons.find(
    (b) => b.text === text && b.visible && b.width > 0 && b.height > 0,
  );
  expect(b, `Native button: ${text}`).toBeTruthy();
  const { width, height } = page.viewportSize();
  if (b.y < 0 || b.y + b.height > height - 64) {
    await page.mouse.move(width - 80, height / 2);
    await page.mouse.wheel(0, b.y - height / 2);
    await frame(page);
    b = (await state(page)).buttons.find(
      (b) => b.text === text && b.visible && b.width > 0,
    );
  }
  await page.mouse.click(b.x + b.width / 2, b.y + b.height / 2);
  await frame(page);
}
async function search(page, value) {
  const s = (await state(page)).searches.find((s) => s.visible);
  expect(s).toBeTruthy();
  await page.mouse.click(s.x + 40, s.y + s.height / 2);
  await page.keyboard.press("ControlOrMeta+A");
  await page.keyboard.type(value);
  await expect.poll(async () => (await state(page)).query).toBe(value);
  await frame(page);
}
async function start(page, path = "/") {
  await page.route("**/ferro-app.json", (r) =>
    r.fulfill({
      status: 404,
      body: "Not adopted yet",
      headers: { "access-control-allow-origin": "*" },
    }),
  );
  await page.goto(path);
  await page.waitForFunction(() => globalThis.ferroStore?.uiState());
  await expect.poll(async () => (await state(page)).apps).toBe(4);
  await expect
    .poll(async () =>
      (await state(page)).buttons.some(
        (b) => b.visible && b.width > 0 && b.height > 0,
      ),
    )
    .toBeTruthy();
}
test("native storefront filters categories, searches, and opens a shareable detail route", async ({
  page,
}) => {
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await start(page);
  await button(page, "Play");
  await expect.poll(async () => (await state(page)).category).toBe("Games");
  await button(page, "Solitaire");
  await expect(page).toHaveURL(/#solitaire$/);
  await expect.poll(async () => (await state(page)).selected).toBe("solitaire");
  await button(page, "←  Back to apps");
  await expect.poll(async () => (await state(page)).selected).toBe(null);
  await button(page, "All apps");
  await search(page, "scientific");
  await button(page, "Calculator");
  await expect(page).toHaveURL(/#calculator$/);
  expect(errors).toEqual([]);
});
test("launches an iframe, closes it with Escape, and offers a separate tab", async ({
  page,
}) => {
  await start(page, "/#calculator");
  await button(page, "Open app");
  await expect(page.getByRole("dialog")).toBeVisible();
  await expect(page.locator("iframe")).toHaveAttribute(
    "src",
    "https://ferroui.github.io/calculator/",
  );
  await expect(
    page.getByRole("link", { name: "Open in new tab" }),
  ).toHaveAttribute("rel", "noopener noreferrer");
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog")).not.toBeVisible();
  await expect(page.locator("iframe")).toHaveCount(0);
  const popup = page.waitForEvent("popup");
  await button(page, "Open in new tab  ↗");
  const tab = await popup;
  await expect(tab).toHaveURL("https://ferroui.github.io/calculator/");
  await tab.close();
});
test("live metadata updates a listing and supports screenshot and video previews", async ({
  page,
}) => {
  await page.route("**/ferro-app.json", (r) =>
    r.fulfill({ status: 404, body: "" }),
  );
  await page.route("**/calculator/ferro-app.json", (r) =>
    r.fulfill({
      json: {
        ...sample,
        name: "Calculator Live",
        description: "The newest description.",
        screenshots: [
          {
            url: "https://ferroui.github.io/calculator/favicon.svg",
            alt: "Standard calculator screenshot",
          },
        ],
        videos: [
          {
            url: "https://ferroui.github.io/calculator/demo.mp4",
            alt: "Calculator tour",
            type: "video/mp4",
          },
        ],
      },
      headers: { "access-control-allow-origin": "*" },
    }),
  );
  await page.goto("/#calculator");
  await page.waitForFunction(() => globalThis.ferroStore?.uiState());
  await expect
    .poll(() =>
      page.evaluate(() => localStorage.getItem("ferro-store:v1:calculator")),
    )
    .toContain("Calculator Live");
  await button(page, "View screenshots & video");
  await expect(page.getByRole("dialog")).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "Calculator Live" }),
  ).toBeVisible();
  await expect(
    page.getByAltText("Standard calculator screenshot"),
  ).toBeVisible();
  await expect(page.locator("video")).toHaveAttribute("controls", "");
  await page.getByRole("button", { name: "Close app preview" }).click();
  await expect(page.getByRole("dialog")).not.toBeVisible();
});
test("mobile search and bottom navigation work at the GitHub Pages project path", async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await start(page, "/FerroStore/#unknown-app");
  expect((await state(page)).selected).toBe(null);
  await search(page, "Ember");
  await button(page, "Ember");
  await expect(page).toHaveURL(/#ember$/);
  await button(page, "Apps");
  await expect.poll(async () => (await state(page)).collection).toBe(true);
  await expect.poll(async () => (await state(page)).query).toBe("");
  await button(page, "Play");
  await expect.poll(async () => (await state(page)).category).toBe("Games");
  await button(page, "Solitaire");
  await expect(page).toHaveURL(/#solitaire$/);
});
test("mobile featured apps scroll horizontally and launch directly", async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await start(page);
  await page.evaluate(() => globalThis.ferroStoreImagesReady());
  const find = async () =>
    (await state(page)).buttons.find((b) => b.text === "Open Solitaire  →");
  expect((await find()).x).toBeGreaterThan(300);
  await page.mouse.move(210, 400);
  await page.mouse.wheel(330, 0);
  await expect.poll(async () => (await find()).x).toBeLessThan(200);
  await frame(page);
  await button(page, "Open Solitaire  →");
  await expect(page.getByRole("dialog")).toBeVisible();
  await expect(page.locator("iframe")).toHaveAttribute(
    "src",
    "https://ferroui.github.io/Solitaire/",
  );
});
test("captures desktop, mobile, detail, and tablet layouts for visual review", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1440, height: 1120 });
  await start(page);
  await page.evaluate(() => globalThis.ferroStoreImagesReady());
  await frame(page);
  await page.mouse.move(20, 1100);
  await page.screenshot({ path: "artifacts/ferro-store-v2-desktop.png" });
  await page.setViewportSize({ width: 390, height: 844 });
  await frame(page);
  await page.screenshot({ path: "artifacts/ferro-store-v2-mobile.png" });
  await page.setViewportSize({ width: 960, height: 900 });
  await frame(page);
  await page.screenshot({ path: "artifacts/ferro-store-v2-tablet.png" });
  await page.setViewportSize({ width: 1440, height: 1120 });
  await page.goto("/#ferrodesign");
  await page.waitForFunction(() => globalThis.ferroStore?.uiState());
  await page.evaluate(() => globalThis.ferroStoreImagesReady());
  await frame(page);
  await page.screenshot({ path: "artifacts/ferro-store-v2-details.png" });
});
