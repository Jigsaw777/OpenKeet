import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { mkdir, mkdtemp, realpath, rm, stat } from "node:fs/promises";
import { createServer as createHttpServer } from "node:http";
import { createServer } from "node:net";
import { join, sep } from "node:path";
import { setTimeout as wait } from "node:timers/promises";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const executable = process.env.COPILOT_DESKTOP_EXE || fileURLToPath(new URL("../../../target/release/OpenKeet.exe", import.meta.url));
const output = fileURLToPath(new URL("../../../target/ui-smoke/", import.meta.url));
await mkdir(output, { recursive: true });
const profile = await mkdtemp(join(output, "webview-"));
const server = createServer();
server.listen(0, "127.0.0.1");
await once(server, "listening");
const { port } = server.address();
await new Promise(resolve => server.close(resolve));
const launchOptions = {
  env: { ...process.env, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port} --remote-debugging-address=127.0.0.1`, WEBVIEW2_USER_DATA_FOLDER: profile, COPILOT_DATA_DIR: join(profile, "application") },
  stdio: "ignore", windowsHide: true,
};
const child = spawn(executable, [], launchOptions);
let spawnError;
child.on("error", error => { spawnError = error; });

async function checkContrast(page, theme) {
  const pairs = await page.evaluate(() => {
    const style = getComputedStyle(document.documentElement);
    return [["--ink", "--surface", 4.5], ["--muted", "--surface", 4.5], ["--rail-muted", "--rail", 4.5],
      ["--primary-ink", "--primary", 4.5], ["--field-border", "--surface", 3], ["--focus", "--canvas", 3]]
      .map(([fg, bg, minimum]) => [style.getPropertyValue(fg).trim(), style.getPropertyValue(bg).trim(), minimum, fg]);
  });
  function luminance(hex) {
    const raw = hex.slice(1);
    const digits = raw.length === 3 ? [...raw].map(char => char.repeat(2)).join("") : raw;
    return digits.match(/../g).reduce((value, digit, index) => {
      const channel = parseInt(digit, 16) / 255;
      const linear = channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4;
      return value + linear * [0.2126, 0.7152, 0.0722][index];
    }, 0);
  }
  for (const [fg, bg, minimum, token] of pairs) {
    const colors = [luminance(fg), luminance(bg)];
    const ratio = (Math.max(...colors) + 0.05) / (Math.min(...colors) + 0.05);
    assert(ratio >= minimum, `${theme} ${token} contrast ${ratio.toFixed(2)} is below ${minimum}.`);
  }
  console.log(`Contrast checks passed for ${theme} text, field borders, and focus tokens.`);
}

let browser;
let fixture;
const requests = [];
const fixtureModel = "copilot-smoke:local";
const fixtureText = "Loopback fixture answer.\n\n- Ground the answer in the question.\n- State assumptions.\n\n```rust\nlet answer = 42;\n```";
let catalogAvailable = true;
let refuseAnswer = false;
async function fixtureAnswer(req, res) {
  if (req.url === "/api/tags") {
    if (!catalogAvailable) { res.writeHead(503).end(); return; }
    res.setHeader("content-type", "application/json");
    res.end(JSON.stringify({ models: [{ name: fixtureModel, size: 1024 }] }));
    return;
  }
  if (req.url !== "/api/chat") { res.writeHead(404).end(); return; }
  let body = "";
  for await (const chunk of req) {
    body += chunk;
    if (body.length > 128_000) { res.writeHead(413).end(); return; }
  }
  requests.push(JSON.parse(body));
  if (refuseAnswer) { res.writeHead(401).end(); return; }
  res.setHeader("content-type", "application/x-ndjson");
  res.write(`${JSON.stringify({ message: { content: fixtureText }, done: false })}\n`);
  await wait(80);
  res.end('{"done":true}\n');
}

async function checkWindow(check, restoreOverlay = false) {
  if (process.env.COPILOT_TEST_WINDOWS_SHELL !== "1") return;
  const script = fileURLToPath(new URL("./window-smoke.ps1", import.meta.url));
  const probe = spawn("powershell.exe", ["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", script,
    "-AppPid", String(child.pid), "-Check", check, ...(restoreOverlay ? ["-RestoreOverlay"] : [])],
  { windowsHide: true, stdio: "inherit" });
  const [code] = await once(probe, "exit");
  assert.equal(code, 0, `Native Windows check failed for ${check}.`);
}

async function relaunch() {
  const duplicate = spawn(executable, [], launchOptions);
  const [code] = await Promise.race([once(duplicate, "exit"), wait(5_000).then(() => [null])]);
  if (code === null) duplicate.kill();
  assert.equal(code, 0, "Reopening the app must reuse the original process.");
}

try {
  if (process.env.COPILOT_TEST_PROVIDER_FIXTURE === "1") {
    fixture = createHttpServer((req, res) => {
      void fixtureAnswer(req, res).catch(() => {
        if (!res.headersSent) res.writeHead(400);
        res.end();
      });
    });
    fixture.listen(11434, "127.0.0.1");
    await once(fixture, "listening");
  }
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
  await page.getByRole("heading", { name: "Finish setup", exact: true }).waitFor();
  await page.getByRole("status").filter({ hasText: /^Not listening$/ }).waitFor();
  assert.equal(await page.getByRole("navigation", { name: "Workspace" }).getByRole("button").count(), 3);
  await page.locator('html[data-theme="dark"]').waitFor({ state: "attached" });
  assert.equal(await page.locator("html").getAttribute("data-theme"), "dark");
  await checkContrast(page, "dark");
  await page.screenshot({ path: join(output, "first-run.png"), animations: "disabled" });
  await page.keyboard.press("Tab");
  assert.equal(await page.evaluate(() => document.activeElement?.textContent?.trim()), "Skip to content");
  await page.keyboard.press("Enter");
  assert.equal(await page.evaluate(() => document.activeElement?.id), "main-content");

  await page.getByRole("button", { name: "Try a sample", exact: true }).click();
  await page.getByRole("heading", { name: "Sample session", exact: true }).waitFor();
  await page.getByRole("button", { name: "Pause", exact: true }).click();
  await page.getByText("Paused", { exact: true }).waitFor();
  await page.getByRole("button", { name: "Resume", exact: true }).click();
  await page.getByText("Revision 2", { exact: true }).waitFor();
  await page.getByText("Session ended", { exact: true }).waitFor();
  await page.getByText("Follow-up to question 1", { exact: true }).waitFor();
  await page.getByText("Checked", { exact: true }).waitFor();
  await page.getByRole("region", { name: "Practice answer text" }).getByText(/Keep the API call outside the mutex/).waitFor();
  await page.getByRole("button", { name: "Show transcript", exact: true }).click();
  const split = await page.getByRole("region", { name: "Practice answer text" }).boundingBox();
  const width = await page.evaluate(() => innerWidth);
  await page.getByRole("button", { name: "Hide transcript", exact: true }).click();
  const expanded = await page.getByRole("region", { name: "Practice answer text" }).boundingBox();
  assert(expanded.width > split.width * 1.4, "Answer view should widen the reading area.");
  assert.equal(await page.evaluate(() => innerWidth), width);
  assert.equal(await page.evaluate(() => document.fullscreenElement), null);
  console.log(`Answer view widened the reading area from ${Math.round(split.width)} to ${Math.round(expanded.width)} CSS px.`);
  await page.screenshot({ path: join(output, "answer-view.png"), animations: "disabled" });
  await page.getByRole("button", { name: "Show transcript", exact: true }).click();
  await page.locator(".live-grid").screenshot({ path: join(output, "live-long-question.png"), animations: "disabled" });
  await page.getByRole("button", { name: "Use full transcript" }).click();
  await page.waitForFunction(() => document.querySelector("#manual-question")?.value.includes("without blocking the coroutine thread"));
  assert.match(await page.getByRole("textbox", { name: "Question", exact: true }).inputValue(), /shared state\nwithout blocking the coroutine thread/);
  await page.getByRole("button", { name: "Copy answer" }).click();
  await page.getByText("Answer copied", { exact: true }).waitFor();

  const focusPromise = context.waitForEvent("page", { timeout: 12_000 });
  await page.getByRole("button", { name: "Overlay", exact: true }).click();
  const focus = await focusPromise;
  focus.on("pageerror", error => errors.push(error.message));
  await focus.getByText("Session ended", { exact: true }).waitFor();
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
  const overlay = await focus.locator(".focus-shell").evaluate(shell => {
    const color = getComputedStyle(shell).backgroundColor;
    return { color, canvas: getComputedStyle(document.documentElement).backgroundColor, opacity: getComputedStyle(shell).opacity };
  });
  assert.deepEqual(overlay, { color: "rgba(17, 18, 20, 0.7)", canvas: "rgba(0, 0, 0, 0)", opacity: "1" });
  const shell = color => focus.waitForFunction(expected =>
    getComputedStyle(document.querySelector(".focus-shell")).backgroundColor === expected, color, { timeout: 3_000 });
  const opacity = focus.getByRole("slider", { name: "Window opacity" });
  await opacity.focus();
  await opacity.press("Home");
  await shell("rgba(17, 18, 20, 0.3)");
  await opacity.press("End");
  await shell("rgb(17, 18, 20)");
  for (let step = 0; step < 6; step++) await opacity.press("ArrowLeft");
  await shell("rgba(17, 18, 20, 0.7)");
  console.log("Overlay opacity slider runs from 30% to solid while text stays opaque.");
  await focus.screenshot({ path: join(output, "focus.png"), animations: "disabled", omitBackground: true });
  await checkWindow("Overlay");
  await focus.getByRole("button", { name: "New session" }).click();
  if (process.env.COPILOT_TEST_WINDOWS_SHELL === "1") {
    await page.getByText("Show with Ctrl+Alt+I", { exact: true }).waitFor();
    await checkWindow("Visible");
    await checkWindow("Close");
    if (process.env.COPILOT_TEST_WINDOWS_SHORTCUT === "1") {
      await checkWindow("Shortcut");
    } else {
      await relaunch();
    }
    await checkWindow("Visible");
  }

  await page.getByLabel("Session title").fill("Kotlin concurrency practice");
  await page.getByRole("button", { name: "Save to history" }).click();
  await page.getByRole("heading", { name: "Saved sessions", exact: true }).waitFor();
  await page.getByRole("searchbox", { name: "Search saved sessions" }).fill("coroutine");
  await page.getByRole("button", { name: "Search", exact: true }).click();
  await page.getByRole("button", { name: /^Kotlin concurrency practice/ }).click();
  await page.getByRole("region", { name: "Saved transcript" }).getByText(/without blocking the coroutine thread/).waitFor();
  await page.getByLabel("Session notes").fill("Review coroutine cancellation.");
  await page.getByRole("button", { name: "Save notes" }).click();
  await page.getByRole("status").filter({ hasText: "Notes saved." }).waitFor();
  await page.getByRole("button", { name: /^Kotlin concurrency practice/ }).click();
  await page.waitForFunction(() => {
    const notes = document.querySelector("#saved-notes");
    return !notes?.disabled && notes?.value === "Review coroutine cancellation.";
  });
  assert(await page.getByRole("button", { name: "Export JSON" }).isEnabled());
  await page.screenshot({ path: join(output, "saved-sessions.png"), animations: "disabled", fullPage: true });
  await page.getByRole("button", { name: "Delete saved session" }).click();
  await page.getByRole("button", { name: "Confirm delete" }).click();
  await page.getByRole("status").filter({ hasText: "Saved session deleted." }).waitFor();

  const workspace = page.getByRole("navigation", { name: "Workspace" });
  const settings = page.getByRole("navigation", { name: "Settings" });
  const audioSettings = async () => {
    await workspace.getByRole("button", { name: "Settings", exact: true }).click();
    await settings.getByRole("button", { name: "Audio and display", exact: true }).click();
  };
  const answersSettings = async () => {
    await workspace.getByRole("button", { name: "Settings", exact: true }).click();
    await settings.getByRole("button", { name: "Answers", exact: true }).click();
  };
  await answersSettings();
  const provider = page.getByLabel("Answer provider", { exact: true });
  await provider.selectOption("chatgpt");
  await page.getByRole("button", { name: "Continue with ChatGPT" }).waitFor();
  await provider.selectOption("claude");
  await page.getByRole("button", { name: "Open Claude Console" }).waitFor();
  assert.equal(await page.getByLabel("Claude Console API key").getAttribute("type"), "password");
  await page.screenshot({ path: join(output, "providers.png"), fullPage: true, animations: "disabled" });
  await audioSettings();
  assert.equal(await page.getByRole("checkbox", { name: "Short follow-up mode" }).isChecked(), false);
  await page.screenshot({ path: join(output, "audio-setup.png"), animations: "disabled" });
  const engine = page.getByLabel("Speech engine", { exact: true });
  assert.equal(await engine.inputValue(), "whisper", "Whisper stays the default speech engine.");
  await engine.selectOption("parakeet");
  await page.getByLabel("Speech model folder", { exact: true }).waitFor();
  await page.getByRole("button", { name: "Back to session" }).click();
  await page.getByRole("button", { name: "Download Parakeet (670 MB)", exact: true }).waitFor();
  await audioSettings();
  await engine.selectOption("whisper");
  await page.getByLabel("Speech model file").fill(join(output, "missing", "ggml-small.en.bin"));
  await page.getByRole("button", { name: "Back to session" }).click();
  const select = page.getByLabel("Listen to", { exact: true });
  const outputOption = select.locator("option").filter({ hasText: /^System audio .* \(default\)$/ });
  await outputOption.waitFor({ state: "attached" });
  assert.equal(await select.inputValue(), await outputOption.getAttribute("value"), "The default output should be preselected.");
  await page.getByRole("button", { name: "Download (488 MB)", exact: true }).waitFor();
  await page.getByRole("alert").filter({ hasText: /^Could not use ggml-small\.en\.bin/ }).waitFor();
  await page.getByRole("heading", { name: "Finish setup", exact: true }).waitFor();
  assert(await page.getByRole("button", { name: "Start listening", exact: true }).isDisabled(), "Start needs a working speech model.");
  await page.screenshot({ path: join(output, "first-run-setup.png"), animations: "disabled" });

  if (process.env.COPILOT_TEST_MODEL && process.env.COPILOT_TEST_WAV) {
    await audioSettings();
    // A model folder selects Parakeet. A .bin file keeps the default Whisper engine.
    const parakeet = (await stat(process.env.COPILOT_TEST_MODEL)).isDirectory();
    if (parakeet) await engine.selectOption("parakeet");
    await page.getByLabel(parakeet ? "Speech model folder" : "Speech model file", { exact: true }).fill(process.env.COPILOT_TEST_MODEL);
    await page.getByLabel("Words to recognize").fill("LRU cache, Java");
    await page.getByRole("button", { name: "Back to session" }).click();
    await select.selectOption(await outputOption.getAttribute("value"));
    await page.getByRole("heading", { name: "Ready to listen", exact: true }).waitFor();
    const startSession = page.getByRole("button", { name: "Start listening", exact: true });
    await page.screenshot({ path: join(output, "ready-to-listen.png"), animations: "disabled" });
    await startSession.click();
    // Start floats the see-through overlay over the call and hides the main window.
    await focus.getByText("Listening", { exact: true }).waitFor();
    await checkWindow("Overlay");
    const playback = spawn("powershell.exe", ["-NoProfile", "-Command",
      "$ErrorActionPreference='Stop'; (New-Object System.Media.SoundPlayer -ArgumentList $env:COPILOT_TEST_WAV).PlaySync()"],
    { windowsHide: true, stdio: "ignore" });
    const playbackExit = once(playback, "exit");
    await focus.getByText("Hearing speech", { exact: true }).waitFor();
    const [exit] = await playbackExit;
    assert.equal(exit, 0, "Synthetic system playback must finish.");
    await focus.getByText("Question heard", { exact: true }).waitFor({ timeout: 15_000 });
    await focus.getByText("Connect answers in Settings to get replies.", { exact: true }).waitFor();
    await focus.screenshot({ path: join(output, "focus-question-heard.png"), animations: "disabled", omitBackground: true });
    console.log("Overlay walked Listening, Hearing speech and Question heard for the played question.");
    await focus.getByRole("button", { name: "Main window" }).click();
    await page.getByRole("region", { name: "Transcript timeline" }).locator(".line.final").filter({ hasText: /cache.*java/i }).waitFor();
    await wait(3_400);
    assert.equal(await page.getByText("Complete", { exact: true }).count(), 0);
    assert.equal(requests.length, 0);
    if (fixture) {
      catalogAvailable = false;
      await answersSettings();
      await provider.selectOption("ollama");
      await page.getByRole("alert").filter({ hasText: "Answer provider is unavailable" }).waitFor();
      assert.equal(await page.getByRole("button", { name: "Use Ollama", exact: true }).isEnabled(), false);
      catalogAvailable = true;
      await page.getByRole("button", { name: "Refresh models", exact: true }).click();
      await page.waitForFunction(() => document.querySelector("#answer-model")?.value === "copilot-smoke:local");
      await page.getByRole("button", { name: "Use Ollama", exact: true }).click();
      await page.getByRole("heading", { name: "Live session", exact: true }).waitFor();
      // Connecting mid-session keeps answers manual, and the status says so.
      await page.getByText("Press Get answer when they finish.", { exact: true }).waitFor();
      await page.getByRole("button", { name: "Get answer", exact: true }).click();
      await page.getByText("Complete", { exact: true }).waitFor();
      const streamedAnswer = page.getByRole("region", { name: "Answer text" });
      await streamedAnswer.getByText("Loopback fixture answer.").waitFor();
      assert.equal(await streamedAnswer.getByRole("listitem").count(), 2);
      assert.equal(await streamedAnswer.locator("pre code").textContent(), "let answer = 42;");
      assert.equal(requests.length, 1);
      assert.equal(requests[0].model, fixtureModel);
      assert.match(requests[0].messages.at(-1).content, /cache.*java/i);
      refuseAnswer = true;
      await page.getByText("Ask a question", { exact: true }).click();
      await page.getByRole("textbox", { name: "Question", exact: true }).fill("Why?");
      await page.getByRole("button", { name: "Ask", exact: true }).click();
      await page.getByRole("alert").filter({ hasText: "The answer provider refused this connection" }).waitFor();
      refuseAnswer = false;
      await page.getByRole("button", { name: "Retry answer", exact: true }).click();
      await page.getByText("Follow-up to question 1", { exact: true }).waitFor();
      await page.getByText("Complete", { exact: true }).waitFor();
      assert.equal(requests.length, 3);
      assert.deepEqual(requests[2].messages.map(item => item.role), ["system", "user", "assistant", "user"]);
      assert.equal(requests[2].messages[2].content, fixtureText);
      assert.equal(requests[2].messages.at(-1).content, "Why?");
      console.log("Provider selection during capture preserved the question. Typed retry recovered from an authentication rejection.");
    } else {
      await page.getByRole("button", { name: "Choose provider", exact: true }).waitFor();
    }
    await page.getByRole("button", { name: "Overlay", exact: true }).click();
    await focus.getByRole("button", { name: "Pause", exact: true }).waitFor();
    await focus.getByRole("button", { name: "Hide", exact: true }).click();
    await checkWindow("Hidden");
    if (process.env.COPILOT_TEST_WINDOWS_SHORTCUT === "1") await checkWindow("Shortcut", true);
    else await relaunch();
    await checkWindow("Overlay");
    await focus.getByText("Paused", { exact: true }).waitFor();
    await focus.getByRole("button", { name: "Resume", exact: true }).click();
    await focus.getByRole("button", { name: "Pause", exact: true }).waitFor();
    await focus.getByRole("button", { name: "Stop", exact: true }).click();
    await focus.getByText("Session ended", { exact: true }).waitFor();
    await focus.getByRole("button", { name: "New session" }).click();
    await startSession.waitFor();
    await page.getByRole("button", { name: "Clear current session" }).click();
    await page.reload();
    await page.getByRole("heading", { name: "Ready to listen", exact: true }).waitFor();
    assert(await startSession.isEnabled(), "Audio setup must survive stop and reload.");
    if (fixture) {
      await page.locator(".checklist").getByText(/Ollama.*copilot-smoke/).waitFor();
      assert(await page.getByRole("checkbox", { name: "Answer detected questions automatically" }).isChecked());
    }
    await page.screenshot({ path: join(output, "session-ready.png"), animations: "disabled" });
    console.log("Audio readiness survived stop and reload. Hiding paused capture and reopening kept it paused.");
  }

  await audioSettings();
  await page.getByLabel("Text size").selectOption("150");
  await workspace.getByRole("button", { name: "Session", exact: true }).click();
  await page.setViewportSize({ width: 650, height: 780 });
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
  await page.screenshot({ path: join(output, "first-run-narrow-150.png"), animations: "disabled" });
  await answersSettings();
  assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), "Providers overflow at 650 px and 150% text.");
  await audioSettings();
  const theme = page.getByLabel("Appearance");
  await theme.selectOption("light");
  await page.locator('html[data-theme="light"]').waitFor({ state: "attached" });
  await checkContrast(page, "light");
  await page.screenshot({ path: join(output, "overview-light.png"), animations: "disabled" });
  await page.emulateMedia({ colorScheme: "dark", reducedMotion: "reduce" });
  await theme.selectOption("system");
  await page.locator('html[data-theme="dark"]').waitFor({ state: "attached" });
  await page.emulateMedia({ colorScheme: "light", reducedMotion: "reduce" });
  await page.locator('html[data-theme="light"]').waitFor({ state: "attached" });
  await theme.selectOption("dark");
  await page.locator('html[data-theme="dark"]').waitFor({ state: "attached" });
  for (const scale of ["100", "125", "150", "200"]) {
    await page.getByLabel("Text size").selectOption(scale);
    for (const [width, height] of [[1366, 768], [1920, 1080], [2560, 1440], [3840, 2160]]) {
      await page.setViewportSize({ width: Math.round(width * 100 / Number(scale)), height: Math.round(height * 100 / Number(scale)) });
      assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), `Audio setup overflow at ${width} × ${height}, ${scale}%.`);
    }
  }
  assert.deepEqual(errors, []);
  console.log("Native WebView2 smoke passed. Screenshots in target/ui-smoke.");
} finally {
  await browser?.close().catch(() => {});
  if (fixture?.listening) {
    fixture.closeAllConnections();
    await new Promise(resolve => fixture.close(resolve));
  }
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
