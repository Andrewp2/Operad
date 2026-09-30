#!/usr/bin/env python3
"""Verify native preedit, commit, undo and cancellation with Fcitx5 Pinyin.

Build --example runtime_ime first. Requires Xephyr, dbus-run-session, xdotool, xprop,
Fcitx5 and its Pinyin addon. All engine configuration and learned data are kept
in a temporary directory, on an isolated X display and D-Bus session.
"""

import os
import pathlib
import queue
import shutil
import subprocess
import sys
import tempfile
import threading
import time


def stop(process):
    process.terminate()
    try:
        process.wait(timeout=5)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait()


def probe(binary, directory):
    with open(directory / "engine.log", "w") as log:
        engine = subprocess.Popen(["fcitx5", "--replace"], stdout=log, stderr=log)
        app = None
        try:
            deadline = time.monotonic() + 15
            while True:
                bus_ready = "boolean true" in subprocess.run([
                    "dbus-send", "--session", "--print-reply", "--dest=org.freedesktop.DBus",
                    "/org/freedesktop/DBus", "org.freedesktop.DBus.NameHasOwner", "string:org.fcitx.Fcitx5",
                ], capture_output=True, text=True, timeout=2).stdout
                xim_ready = "@server=fcitx" in subprocess.run(
                    ["xprop", "-root", "XIM_SERVERS"],
                    capture_output=True, text=True, timeout=2,
                ).stdout
                if bus_ready and xim_ready:
                    break
                if engine.poll() is not None or time.monotonic() >= deadline:
                    raise AssertionError("Fcitx5 D-Bus/XIM startup did not complete")
                time.sleep(0.05)
            app = subprocess.Popen([str(binary)], stdout=subprocess.PIPE, text=True, bufsize=1)
            lines = queue.Queue()

            def read():
                for line in app.stdout:
                    lines.put(line)
                lines.put(None)

            threading.Thread(target=read, daemon=True).start()

            def wait(predicate, description):
                deadline = time.monotonic() + 15
                last = None
                while time.monotonic() < deadline:
                    try:
                        line = lines.get(timeout=0.1)
                    except queue.Empty:
                        continue
                    if line is None:
                        raise AssertionError(f"Native fixture exited: {app.poll()}")
                    last = line.strip()
                    if predicate(line):
                        return
                raise AssertionError(f"{description}: {last}")

            def xdo(*args):
                return subprocess.check_output(["xdotool", *args], text=True).strip()

            wait(lambda line: "IME_STATE" in line, "Startup")
            window = xdo("search", "--onlyvisible", "--name", "^Text composition$").splitlines()[0]
            xdo("windowfocus", window)
            xdo("mousemove", "--window", window, "55", "50")
            xdo("click", "1")
            wait(lambda line: "focused=true" in line and "pressed=false" in line,
                 "Processed field focus and pointer release")
            xdo("key", "ctrl+a")
            wait(lambda line: "anchor=Some(0)" in line and "caret=9 " in line,
                 "Select all before composition")
            # Activation before the app processes focus can target Fcitx's
            # fallback context. A successful remote command alone is not an ack.
            subprocess.run(["fcitx5-remote", "-s", "pinyin"], check=True)
            subprocess.run(["fcitx5-remote", "-o"], check=True)
            deadline = time.monotonic() + 15
            while True:
                active = subprocess.check_output(["fcitx5-remote"], text=True, timeout=2).strip()
                method = subprocess.check_output(["fcitx5-remote", "-n"], text=True, timeout=2).strip()
                if active == "2" and method == "pinyin":
                    break
                if engine.poll() is not None or time.monotonic() >= deadline:
                    raise AssertionError(f"Pinyin activation failed: state={active!r}, method={method!r}")
                time.sleep(0.05)
            xdo("type", "--clearmodifiers", "--delay", "120", "nihao")
            wait(lambda line: 'display="ni hao"' in line and "composing=true" in line,
                 "Inline Pinyin preedit")
            if os.environ.get("OPERAD_IME_SCREENSHOT"):
                subprocess.run(["import", "-window", "root", os.environ["OPERAD_IME_SCREENSHOT"]], check=True)
            xdo("key", "space")
            wait(lambda line: 'committed="你好"' in line and "composing=false" in line and "commits=1" in line,
                 "Single native commit")
            xdo("key", "ctrl+z")
            wait(lambda line: 'committed="a😀旧z"' in line and "composing=false" in line,
                 "Undo restores original selection replacement")
            xdo("key", "ctrl+a")
            wait(lambda line: "anchor=Some(0)" in line and "caret=9 " in line,
                 "Select all before second composition")
            xdo("type", "--clearmodifiers", "--delay", "120", "nihao")
            wait(lambda line: 'display="ni hao"' in line and "composing=true" in line, "Second preedit")
            xdo("key", "Escape")
            wait(lambda line: 'committed="a😀旧z"' in line and "composing=false" in line and "commits=2" in line,
                 "Cancel preserves committed text/history")
            print("Native IME probe passed: inline Pinyin, one commit, undo, cancellation.")
        except Exception:
            log.flush()
            print("Fcitx5 log:\n" + (directory / "engine.log").read_text()[-8000:], file=sys.stderr)
            raise
        finally:
            if app is not None:
                stop(app)
            stop(engine)


def main():
    if len(sys.argv) > 1 and sys.argv[1] == "--inside":
        probe(pathlib.Path(sys.argv[2]), pathlib.Path(sys.argv[3]))
        return
    for command in ["Xephyr", "dbus-run-session", "dbus-send", "fcitx5", "fcitx5-remote", "xdotool", "xprop"]:
        if not shutil.which(command):
            raise SystemExit(f"Native IME probe requires {command}")
    root = pathlib.Path(__file__).resolve().parent.parent
    binary = pathlib.Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else root / "target/debug/examples/runtime_ime"
    display = next(f":{n}" for n in range(97, 128) if not pathlib.Path(f"/tmp/.X11-unix/X{n}").exists())
    with tempfile.TemporaryDirectory(prefix="operad-ime-") as temporary:
        directory = pathlib.Path(temporary)
        runtime = directory / "runtime"
        runtime.mkdir(mode=0o700)
        config = directory / "config/fcitx5"
        (config / "conf").mkdir(parents=True)
        (config / "profile").write_text("[Groups/0]\nName=Default\nDefault Layout=us\nDefaultIM=pinyin\n\n[Groups/0/Items/0]\nName=keyboard-us\nLayout=\n\n[Groups/0/Items/1]\nName=pinyin\nLayout=\n\n[GroupOrder]\n0=Default\n")
        (config / "config").write_text("[Behavior]\nPreeditEnabledByDefault=True\n")
        (config / "conf/xim.conf").write_text("UseOnTheSpot=True\n")
        with open(directory / "display.log", "w") as log:
            display_process = subprocess.Popen(["Xephyr", display, "-screen", "1024x768", "-ac", "-nolisten", "tcp"], stdout=log, stderr=log)
            try:
                deadline = time.monotonic() + 10
                while not pathlib.Path(f"/tmp/.X11-unix/X{display[1:]}").exists():
                    if display_process.poll() is not None or time.monotonic() > deadline:
                        raise AssertionError("Xephyr did not start")
                    time.sleep(0.05)
                env = dict(os.environ, DISPLAY=display, XMODIFIERS="@im=fcitx",
                           XDG_CONFIG_HOME=str(directory / "config"),
                           XDG_RUNTIME_DIR=str(runtime),
                           XDG_DATA_HOME=str(directory / "data"), XDG_CACHE_HOME=str(directory / "cache"))
                env.pop("WAYLAND_DISPLAY", None)
                env.pop("WAYLAND_SOCKET", None)
                env.pop("FCITX_NO_PREEDIT_APPS", None)
                subprocess.run(["dbus-run-session", "--", sys.executable, __file__, "--inside", str(binary), str(directory)], env=env, check=True, timeout=90)
            finally:
                stop(display_process)


if __name__ == "__main__":
    main()
