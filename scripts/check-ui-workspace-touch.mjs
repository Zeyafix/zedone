import assert from "node:assert/strict";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";
import { checkWorkspaceTouch } from "./check-ui-android-scenarios.mjs";

/** Exercise the shared Android selectors with host touch and an empty Service.
 * Native IME/Back remains a separate Android check, not simulated by this proof. */
export async function checkWorkspaceTouchHost(browser, executable) {
  const directory = await mkdtemp(resolve(tmpdir(), "zedone-workspace-touch-"));
  const fixture = Bun.spawn([executable, "--serve", directory], { stdout: "pipe", stderr: "inherit" });
  let context;
  try {
    const reader = fixture.stdout.getReader();
    let address = "";
    while (!address.includes("\n")) {
      const { value, done } = await reader.read();
      assert.ok(!done, "touch fixture exited before readiness");
      address += new TextDecoder().decode(value);
    }
    reader.releaseLock();
    const url = address.trim();
    assert.match(url, /^http:\/\/127\.0\.0\.1:\d+$/);
    context = await browser.newContext({ viewport: { width: 360, height: 760 }, colorScheme: "dark", hasTouch: true, isMobile: true });
    const page = await context.newPage(), errors = [];
    page.on("pageerror", error => errors.push(error.message));
    await page.goto(url);
    await checkWorkspaceTouch(page, async () => {
      if (await page.evaluate(() => !!document.querySelector('[role="dialog"]'))) await page.keyboard.press("Escape");
      else await page.goBack();
    });
    assert.deepEqual(errors, []);
    console.log("Shared Android workspace touch scenario passed against host HTTP/Service; native IME/Back is checked separately.");
  } finally {
    await context?.close();
    fixture.kill(); await fixture.exited;
    assert.ok(directory.startsWith(resolve(tmpdir(), "zedone-workspace-touch-")));
    await rm(directory, { recursive: true });
  }
}
