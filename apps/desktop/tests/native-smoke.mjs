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
async function localPort() {
  const server = createServer();
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  const { port } = server.address();
  await new Promise(resolve => server.close(resolve));
  return port;
}
const port = await localPort();
const bridgePort = process.env.COPILOT_TEST_PROVIDER_FIXTURE === "1" ? await localPort() : null;
const bridge = bridgePort ? `http://127.0.0.1:${bridgePort}` : null;
const launchOptions = {
  env: { ...process.env, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port} --remote-debugging-address=127.0.0.1`, WEBVIEW2_USER_DATA_FOLDER: profile, COPILOT_DATA_DIR: join(profile, "application") },
  stdio: "ignore", windowsHide: true,
};
const child = spawn(executable, bridgePort ? ["--text-bridge", String(bridgePort)] : [], launchOptions);
let spawnError;
child.on("error", error => { spawnError = error; });

async function checkContrast(page, theme) {
  const pairs = await page.evaluate(() => {
    const style = getComputedStyle(document.documentElement);
    return [["--ink", "--surface", 4.5], ["--muted", "--surface", 4.5], ["--rail-muted", "--rail", 4.5],
      ["--primary-ink", "--primary", 4.5], ["--accent", "--soft", 4.5], ["--field-border", "--surface", 3], ["--focus", "--canvas", 3]]
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
const fixtureText = "Loopback fixture answer.\n\nStart with the constraint that matters most, then explain your approach with one concrete example from your own work.\n\nClose with the main trade-off and how you would check it in production before rolling it out widely.\n\n- Ground the answer in the question.\n- State assumptions.\n\n```rust\nlet answer = 42;\n```";
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

async function checkWindow(check, restoreOverlay = false, captureAffinity) {
  if (process.env.COPILOT_TEST_WINDOWS_SHELL !== "1") return;
  const script = fileURLToPath(new URL("./window-smoke.ps1", import.meta.url));
  const probe = spawn("powershell.exe", ["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", script,
    "-AppPid", String(child.pid), "-Check", check, ...(restoreOverlay ? ["-RestoreOverlay"] : []),
    ...(captureAffinity === undefined ? [] : ["-CaptureAffinity", String(captureAffinity)])],
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
  if (bridge) {
    assert.deepEqual(await fetch(`${bridge}/api/tags`).then(response => response.json()), { models: [] });
    assert.equal((await fetch(`${bridge}/api/tags`, { headers: { Origin: "https://other.example" } })).status, 403);
  }
  const workspace = page.getByRole("navigation", { name: "Workspace" });
  assert.deepEqual(await workspace.getByRole("button").allTextContents(), [
    "Prepare", "My experience", "Practice", "Session", "Community", "Settings", "History",
  ]);
  await page.locator('html[data-theme="dark"]').waitFor({ state: "attached" });
  assert.equal(await page.locator("html").getAttribute("data-theme"), "dark");
  await checkContrast(page, "dark");
  await page.screenshot({ path: join(output, "first-run.png"), animations: "disabled" });
  await page.keyboard.press("Tab");
  assert.equal(await page.evaluate(() => document.activeElement?.textContent?.trim()), "Skip to content");
  await page.keyboard.press("Enter");
  assert.equal(await page.evaluate(() => document.activeElement?.id), "main-content");

  let focus;
  let ollamaConnected = false;

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
  const capturePrivacy = page.getByRole("checkbox", { name: "Screen-share privacy" });
  if (process.platform === "win32") {
    assert.equal(await capturePrivacy.isChecked(), false, "Screen-share privacy starts off.");
    if (fixture) {
      await capturePrivacy.click();
      await capturePrivacy.and(page.locator(":checked")).waitFor();
    }
  }
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
    const overlayPage = context.waitForEvent("page", { timeout: 12_000 });
    await startSession.click();
    // Start floats the see-through overlay over the call and hides the main window.
    focus = await overlayPage;
    focus.on("pageerror", error => errors.push(error.message));
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
      ollamaConnected = true;
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

  // A typed question answered by the local fixture provider drives the answer view, overlay, window and history checks.
  if (fixture) {
    if (!ollamaConnected) {
      await answersSettings();
      await provider.selectOption("ollama");
      await page.waitForFunction(() => document.querySelector("#answer-model")?.value === "copilot-smoke:local");
      await page.getByRole("button", { name: "Use Ollama", exact: true }).click();
    }
    await workspace.getByRole("button", { name: "Session", exact: true }).click();
    await page.getByText("Ask a question", { exact: true }).click();
    await page.getByRole("textbox", { name: "Question", exact: true }).fill("How would you protect shared state without blocking the coroutine thread?");
    await page.getByRole("button", { name: "Ask", exact: true }).click();
    const answerText = page.getByRole("region", { name: "Answer text" });
    await answerText.getByText("Loopback fixture answer.").waitFor();
    await page.getByText("Complete", { exact: true }).waitFor();
    assert.match(requests.at(-1).messages.at(-1).content, /coroutine thread/);
    await page.getByRole("button", { name: "Show transcript", exact: true }).click();
    const split = await answerText.boundingBox();
    const width = await page.evaluate(() => innerWidth);
    await page.getByRole("button", { name: "Hide transcript", exact: true }).click();
    const expanded = await answerText.boundingBox();
    assert(expanded.width > split.width * 1.4, "Answer view should widen the reading area.");
    assert.equal(await page.evaluate(() => innerWidth), width);
    assert.equal(await page.evaluate(() => document.fullscreenElement), null);
    console.log(`Answer view widened the reading area from ${Math.round(split.width)} to ${Math.round(expanded.width)} CSS px.`);
    await page.screenshot({ path: join(output, "answer-view.png"), animations: "disabled" });
    await page.getByRole("button", { name: "Show transcript", exact: true }).click();
    await page.locator(".live-grid").screenshot({ path: join(output, "live-long-question.png"), animations: "disabled" });
    await page.getByRole("button", { name: "Copy answer" }).click();
    await page.getByText("Answer copied", { exact: true }).waitFor();

    const overlayPage = focus ? null : context.waitForEvent("page", { timeout: 12_000 });
    await page.getByRole("button", { name: "Overlay", exact: true }).click();
    if (overlayPage) {
      focus = await overlayPage;
      focus.on("pageerror", error => errors.push(error.message));
    }
    await focus.locator(".compact-question summary").filter({ hasText: /coroutine thread/ }).waitFor();
    await focus.setViewportSize({ width: 380, height: 480 });
    const reading = focus.getByRole("region", { name: "Answer text" });
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
    assert.deepEqual(overlay, { color: "rgba(11, 16, 32, 0.7)", canvas: "rgba(0, 0, 0, 0)", opacity: "1" });
    const shell = color => focus.waitForFunction(expected =>
      getComputedStyle(document.querySelector(".focus-shell")).backgroundColor === expected, color, { timeout: 3_000 });
    const opacity = focus.getByRole("slider", { name: "Window opacity" });
    await opacity.focus();
    await opacity.press("Home");
    await shell("rgba(11, 16, 32, 0.3)");
    await opacity.press("End");
    await shell("rgb(11, 16, 32)");
    for (let step = 0; step < 6; step++) await opacity.press("ArrowLeft");
    await shell("rgba(11, 16, 32, 0.7)");
    console.log("Overlay opacity slider runs from 30% to solid while text stays opaque.");
    await focus.screenshot({ path: join(output, "focus.png"), animations: "disabled", omitBackground: true });
    await checkWindow("Overlay", false, 0x11);
    await focus.getByRole("button", { name: "New session" }).click();
    if (process.env.COPILOT_TEST_WINDOWS_SHELL === "1") {
      await page.getByText("Show with Ctrl+Alt+I", { exact: true }).waitFor();
      await checkWindow("Visible", false, 0x11);
      await checkWindow("Close");
      if (process.env.COPILOT_TEST_WINDOWS_SHORTCUT === "1") {
        await checkWindow("Shortcut");
      } else {
        await relaunch();
      }
      await checkWindow("Visible", false, 0x11);
    }
    if (process.platform === "win32") {
      await audioSettings();
      assert.equal(await capturePrivacy.isChecked(), true, "Privacy stays on when returning to the workspace.");
      await capturePrivacy.click();
      await capturePrivacy.and(page.locator(":not(:checked)")).waitFor();
      await page.getByRole("button", { name: "Back to session" }).click();
      await checkWindow("Visible", false, 0);
      await page.getByRole("button", { name: "Overlay", exact: true }).click();
      await checkWindow("Overlay", false, 0);
      await focus.getByRole("button", { name: "New session" }).click();
    }

    await page.getByLabel("Session title").fill("Kotlin concurrency practice");
    await page.getByRole("button", { name: "Save to history" }).click();
    await page.getByRole("heading", { name: "Saved sessions", exact: true }).waitFor();
    await page.getByRole("searchbox", { name: "Search saved sessions" }).fill("Kotlin");
    await page.getByRole("button", { name: "Search", exact: true }).click();
    await page.getByRole("button", { name: /^Kotlin concurrency practice/ }).click();
    await page.getByText("Checked answers", { exact: true }).click();
    await page.locator(".saved-answer").getByText(/without blocking the coroutine thread/).waitFor();
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
    console.log("A typed question drove the answer view, overlay, window checks and saved history.");

    const catalog = await fetch(`${bridge}/api/tags`).then(response => response.json());
    assert.deepEqual(catalog.models.map(model => model.name), [fixtureModel]);
    const generate = body => fetch(`${bridge}/api/generate`, {
      method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(body),
    });
    assert.equal((await generate({ prompt: "Describe this image", images: ["example"] })).status, 400);
    assert.equal((await generate({ prompt: "x".repeat(150_000) })).status, 413);
    const prompt = "What makes an HTTP request idempotent?";
    const complete = await generate({ prompt, model: fixtureModel });
    assert.equal(complete.status, 200);
    assert.deepEqual(await complete.json(), { model: fixtureModel, response: fixtureText, done: true });
    const streaming = await generate({ prompt, model: fixtureModel, stream: true });
    assert.equal(streaming.headers.get("content-type"), "application/x-ndjson");
    const frames = (await streaming.text()).trim().split("\n").map(line => JSON.parse(line));
    assert.equal(frames.filter(frame => !frame.done).map(frame => frame.response).join(""), fixtureText);
    assert.equal(frames.at(-1).done, true);
    assert.equal(requests.at(-1).messages.at(-1).content, prompt);
    console.log("The Rust text bridge served the selected model, JSON and streaming answers, and rejected invalid requests.");
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
  await workspace.getByRole("button", { name: "Prepare", exact: true }).click();
  const upload = page.getByLabel("Choose a file", { exact: true });
  await upload.setInputFiles({
    name: "profile.json", mimeType: "application/json",
    buffer: Buffer.from(JSON.stringify({
      resume: "Experience\nEngineer, Example Co\n- Built a Redis cache for the catalog API",
      job: "Backend engineer with Redis experience",
      notes: "Redis",
      provider: "Claude Code (subscription)",
    })),
  });
  await page.getByRole("button", { name: "Add document", exact: true }).click();
  await page.getByRole("status").filter({ hasText: "Legacy profile documents imported." }).waitFor();
  for (const title of ["Resume", "Job description", "Project notes"]) {
    await page.getByText(`${title} from legacy profile`, { exact: true }).waitFor();
  }
  console.log("Legacy profile imported all three local documents, including short notes.");
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
