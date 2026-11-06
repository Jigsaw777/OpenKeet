import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { mkdir, mkdtemp, realpath, rm } from "node:fs/promises";
import { createServer } from "node:net";
import { join, sep } from "node:path";
import { setTimeout as wait } from "node:timers/promises";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const executable = process.env.COPILOT_DESKTOP_EXE || fileURLToPath(new URL("../../../target/release/personal-copilot-desktop.exe", import.meta.url));
const output = fileURLToPath(new URL("../../../target/ui-smoke/", import.meta.url));
await mkdir(output, { recursive: true });
const profile = await mkdtemp(join(output, "webview-"));
const server = createServer();
server.listen(0, "127.0.0.1");
await once(server, "listening");
const { port } = server.address();
await new Promise(resolve => server.close(resolve));
const child = spawn(executable, [], {
  env: { ...process.env, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port} --remote-debugging-address=127.0.0.1`, WEBVIEW2_USER_DATA_FOLDER: profile },
  stdio: "ignore", windowsHide: true,
});
let spawnError;
child.on("error", error => { spawnError = error; });

let browser;
try {
  const endpoint = `http://127.0.0.1:${port}`;
  const deadline = Date.now() + 20_000;
  while (Date.now() < deadline) {
    if (spawnError) throw spawnError;
    if (child.exitCode !== null) throw new Error(`Desktop process exited with ${child.exitCode}`);
    const available = await fetch(`${endpoint}/json/version`).then(response => response.ok, () => false);
    if (available) break;
    await wait(150);
  }
  browser = await chromium.connectOverCDP(endpoint, { timeout: 12_000 });
  const context = browser.contexts()[0];
  const page = context.pages()[0] || await context.waitForEvent("page");
  const errors = [];
  page.on("pageerror", error => errors.push(error.message));
  await page.getByRole("heading", { name: "Make room for better answers." }).waitFor();
  await page.locator('html[data-theme="dark"]').waitFor({ state: "attached" });
  assert.equal(await page.locator("html").getAttribute("data-theme"), "dark");
  await page.screenshot({ path: join(output, "first-run.png"), animations: "disabled" });
  await page.keyboard.press("Tab");
  assert.equal(await page.evaluate(() => document.activeElement?.textContent?.trim()), "Skip to content");
  await page.keyboard.press("Enter");
  assert.equal(await page.evaluate(() => document.activeElement?.id), "main-content");

  await page.getByRole("button", { name: "Run sample session" }).click();
  await page.getByText("Playing sample", { exact: true }).waitFor();
  await page.getByRole("button", { name: "Pause", exact: true }).click();
  await page.getByText("Paused", { exact: true }).waitFor();
  await page.getByRole("button", { name: "Resume", exact: true }).click();
  await page.getByText("Revision 2", { exact: true }).waitFor();
  await page.getByText("Session ended", { exact: true }).waitFor();
  await page.getByText("Follow-up to question 1", { exact: true }).waitFor();
  await page.getByText("Checked", { exact: true }).waitFor();
  await page.getByRole("region", { name: "Practice answer text" }).getByText(/Keep the API call outside the mutex/).waitFor();
  const split = await page.getByRole("region", { name: "Practice answer text" }).boundingBox();
  const width = await page.evaluate(() => innerWidth);
  await page.getByRole("button", { name: "Answer view", exact: true }).click();
  const expanded = await page.getByRole("region", { name: "Practice answer text" }).boundingBox();
  assert(expanded.width > split.width * 1.4, "Answer view should widen the reading area.");
  assert.equal(await page.evaluate(() => innerWidth), width);
  assert.equal(await page.evaluate(() => document.fullscreenElement), null);
  console.log(`Answer view widened the reading area from ${Math.round(split.width)} to ${Math.round(expanded.width)} CSS px.`);
  await page.screenshot({ path: join(output, "answer-view.png"), animations: "disabled" });
  await page.getByRole("button", { name: "Split view", exact: true }).click();
  await page.screenshot({ path: join(output, "live-long-question.png"), animations: "disabled" });
  await page.getByRole("button", { name: "Use full transcript" }).click();
  await page.waitForFunction(() => document.querySelector("#manual-question")?.value.includes("without blocking the coroutine thread"));
  assert.match(await page.getByRole("textbox", { name: "Ask a practice question" }).inputValue(), /shared state\nwithout blocking the coroutine thread/);
  await page.getByRole("button", { name: "Copy answer" }).click();
  await page.getByText("Answer copied", { exact: true }).waitFor();

  const focusPromise = context.waitForEvent("page", { timeout: 12_000 });
  await page.getByRole("button", { name: "Focus window" }).click();
  const focus = await focusPromise;
  await focus.getByRole("heading", { name: "Focus", exact: true }).waitFor();
  await focus.getByText("Follow-up to question 1", { exact: true }).waitFor();
  await focus.setViewportSize({ width: 380, height: 480 });
  const reading = focus.getByRole("region", { name: "Practice answer text" });
  const visible = await reading.boundingBox();
  assert(visible.height >= 240, `Compact answer only received ${visible.height} px of 480 px.`);
  assert.equal(await focus.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
  await reading.focus();
  await reading.press("End");
  await focus.waitForFunction(() => document.querySelector(".answer-content")?.scrollTop > 0);
  const copy = await focus.getByRole("button", { name: "Copy answer" }).boundingBox();
  assert(copy.y + copy.height <= 480, "Copy should stay in view while the answer scrolls.");
  await reading.press("Home");
  await focus.waitForFunction(() => document.querySelector(".answer-content")?.scrollTop === 0);
  console.log(`Compact answer used ${Math.round(visible.height)} of 480 CSS px with its controls visible.`);
  await focus.screenshot({ path: join(output, "focus.png"), animations: "disabled" });
  await focus.getByRole("button", { name: "Pin", exact: true }).click();
  await focus.getByRole("button", { name: "Unpin", exact: true }).click();
  await focus.getByRole("button", { name: "Open workspace" }).click();

  await page.getByRole("button", { name: "Audio setup", exact: true }).click();
  const select = page.getByLabel("Input route");
  await select.locator("option").nth(1).waitFor({ state: "attached" });
  await select.selectOption({ index: 1 });
  await page.getByLabel("Speech model file").fill(join(output, "missing", "ggml-small.en.bin"));
  assert.equal(await page.getByRole("button", { name: "Start local capture" }).isEnabled(), false);
  await page.getByRole("checkbox", { name: "I have permission to capture this source." }).check();
  await page.getByRole("button", { name: "Start local capture" }).click();
  await page.getByRole("alert").filter({ hasText: "failed verification" }).waitFor();
  await page.getByRole("status").filter({ hasText: "Needs attention" }).waitFor();
  await page.getByRole("button", { name: "Clear current session" }).click();
  await page.getByText("Waiting for speech", { exact: true }).waitFor();

  if (process.env.COPILOT_TEST_MODEL && process.env.COPILOT_TEST_WAV) {
    await page.getByRole("button", { name: "Audio setup", exact: true }).click();
    const output = select.locator("option").filter({ hasText: /^System output .* \(default\)$/ });
    await select.selectOption(await output.getAttribute("value"));
    await page.getByLabel("Speech model file").fill(process.env.COPILOT_TEST_MODEL);
    await page.getByLabel("Optional vocabulary").fill("LRU cache, Java");
    await page.getByRole("checkbox", { name: "I have permission to capture this source." }).check();
    await page.getByRole("button", { name: "Start local capture" }).click();
    await page.getByRole("status").filter({ hasText: /^Listening$/ }).waitFor();
    const playback = spawn("powershell.exe", ["-NoProfile", "-Command",
      "$ErrorActionPreference='Stop'; (New-Object System.Media.SoundPlayer -ArgumentList $env:COPILOT_TEST_WAV).PlaySync()"],
    { windowsHide: true, stdio: "ignore" });
    const [exit] = await once(playback, "exit");
    assert.equal(exit, 0, "Synthetic system playback must finish.");
    await page.getByRole("region", { name: "Transcript timeline" }).locator(".line.final").filter({ hasText: /cache.*java/i }).waitFor();
    await page.getByText("Checked", { exact: true }).waitFor();
    await page.getByRole("button", { name: "Stop", exact: true }).click();
    await page.getByRole("status").filter({ hasText: /^Session ended$/ }).waitFor();
    await page.getByRole("button", { name: "Clear current session" }).click();
    console.log("Installed app captured the synthetic WASAPI clip and checked its question.");
  }

  await page.getByRole("button", { name: "Audio setup", exact: true }).click();
  await page.getByLabel("Text size").selectOption("150");
  await page.getByRole("button", { name: "Overview", exact: true }).click();
  await page.setViewportSize({ width: 650, height: 780 });
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
  await page.screenshot({ path: join(output, "first-run-narrow-150.png"), animations: "disabled" });
  const theme = page.getByRole("button", { name: "Toggle light or dark theme" });
  await theme.click();
  await page.locator('html[data-theme="light"]').waitFor({ state: "attached" });
  await theme.click();
  await page.locator('html[data-theme="dark"]').waitFor({ state: "attached" });
  assert.deepEqual(errors, []);
  console.log("Native WebView2 smoke passed. Screenshots in target/ui-smoke.");
} finally {
  await browser?.close().catch(() => {});
  if (child.exitCode === null && !spawnError) {
    child.kill();
    await Promise.race([once(child, "exit"), wait(3_000)]);
  }
  const actual = await realpath(profile);
  const parent = await realpath(output);
  if (actual.startsWith(`${parent}${sep}webview-`)) {
    await rm(actual, { recursive: true, force: true, maxRetries: 3 }).catch(() => {});
  }
}
