import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { once } from "node:events";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";

const runner = fileURLToPath(new URL("./check-web-showcase.mjs", import.meta.url));
const fixture = new URL("../tests/fixtures/cdp-browser.mjs", import.meta.url);

for (const [mode, diagnostic] of [
  ["stall-open", /CDP connection timed out/i],
  ["stall-command", /Target\.createTarget.*timed out/i],
  ["close-command", /Target\.createTarget.*clos/i],
  ["metadata-close", /Target\.createTarget.*clos/i],
  ["invalid-message", /(?:invalid|malformed).*CDP|CDP.*(?:invalid|malformed)/i],
  ["startup-body", /Chrome.*(?:endpoint|timed out)/i],
  ["early-exit", /Chrome.*(?:exit|startup failure)/i],
  ["spawn-error", /EACCES|permission denied/i],
]) {
  test(`browser runner fails and cleans up after ${mode}`, { skip: process.platform === "win32" }, async () => {
    const directory = await fs.mkdtemp(path.join(os.tmpdir(), "operad-cdp-test-"));
    const chromePath = path.join(directory, "fake-chrome");
    const pidPath = path.join(directory, "chrome.pid");
    await fs.copyFile(fixture, chromePath);
    await fs.chmod(chromePath, mode === "spawn-error" ? 0o600 : 0o700);
    const child = spawn(process.execPath, [runner, "http://127.0.0.1:1/unused"], {
      detached: true,
      stdio: ["ignore", "pipe", "pipe"],
      env: {
        ...process.env,
        CHROME_BIN: chromePath,
        TMPDIR: directory,
        DISPLAY: ":fake-browser-regression",
        OPERAD_FAKE_CHROME_MODE: mode,
        OPERAD_FAKE_CHROME_PID: pidPath,
        OPERAD_WEB_SMOKE_TIMEOUT_MS: "1000",
      },
    });
    let output = "";
    child.stdout.on("data", data => { output += data; });
    child.stderr.on("data", data => { output += data; });
    let timedOut = false;
    const killOwnedGroup = () => {
      try { process.kill(-child.pid, "SIGKILL"); }
      catch (error) { if (error.code !== "ESRCH") throw error; }
    };
    const timer = setTimeout(() => { timedOut = true; killOwnedGroup(); }, 5000);
    try {
      const [code] = await once(child, "close");
      assert.equal(timedOut, false, `runner hung after ${mode}: ${output}`);
      assert.equal(code, 1, output);
      assert.match(output, diagnostic);
      const files = await fs.readdir(directory);
      assert.deepEqual(files.filter(name => name.startsWith("operad-web-smoke-")), [],
        `runner leaked its Chrome profile after ${mode}`);
      if (files.includes("chrome.pid")) {
        const pid = Number(await fs.readFile(pidPath, "utf8"));
        assert.throws(() => process.kill(pid, 0), { code: "ESRCH" },
          `fake Chrome ${pid} survived runner failure`);
      }
    } finally {
      clearTimeout(timer);
      killOwnedGroup();
      await fs.rm(directory, { recursive: true, force: true });
    }
  });
}
