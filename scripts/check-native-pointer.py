#!/usr/bin/env python3
"""Exercise showcase checkboxes through native X11 input on a hidden Xvfb display.

Build --features inspector --example native_pointer first. Requires Xvfb and
xdotool; XVFB_BIN may point to an unpacked Xvfb binary. No desktop display is used.
"""

import json
import os
import pathlib
import queue
import select
import shutil
import subprocess
import sys
import tempfile
import threading
import time


def stop(process):
    if process.poll() is None:
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()


def probe(binary, env, directory):
    with open(directory / "app.log", "w") as log:
        app = subprocess.Popen([str(binary)], env=env, stdout=subprocess.PIPE,
                               stderr=log, text=True, bufsize=1)
        states = queue.Queue()

        def read():
            for line in app.stdout:
                log.write(line)
                log.flush()
                if line.startswith("POINTER_STATE "):
                    states.put(json.loads(line.removeprefix("POINTER_STATE ")))
            states.put(None)

        reader = threading.Thread(target=read, daemon=True)
        reader.start()
        current = None

        def wait(predicate, description):
            nonlocal current
            deadline = time.monotonic() + 15
            while time.monotonic() < deadline:
                try:
                    state = states.get(timeout=0.1)
                except queue.Empty:
                    continue
                if state is None:
                    raise AssertionError(f"Native fixture exited: {app.poll()}")
                current = state
                if predicate(state):
                    return state
            raise AssertionError(f"{description}: {current}")

        def xdo(*args):
            return subprocess.check_output(["xdotool", *map(str, args)], env=env, text=True).strip()

        def settle():
            nonlocal current
            # Native rendering may coalesce press/release and hover animation.
            # Always wait for observed input first, then collect later snapshots.
            time.sleep(0.1)
            while not states.empty():
                state = states.get_nowait()
                if state is None:
                    raise AssertionError(f"Native fixture exited: {app.poll()}")
                current = state

        position = None

        def move(name):
            nonlocal position
            point = tuple(round(n) for n in current["points"][name])
            if point != position:
                before = current["processed"]
                xdo("mousemove", "--sync", "--window", window, *point)
                wait(lambda s: s["processed"] > before, f"move to {name}")
                settle()
                position = point

        def click(button):
            before = current["processed"]
            xdo("click", button)
            wait(lambda s: s["processed"] > before, f"button {button} input")
            settle()

        def values():
            return current["checked"], current["open"]

        try:
            wait(lambda s: len(s["points"]) == 4, "first showcase frame")
            window = xdo("search", "--onlyvisible", "--name", "^Native pointer probe$").splitlines()[0]
            xdo("windowfocus", window)
            for name in ["checkbox.enabled", "controls.checkbox"]:
                for _ in range(2):
                    for part in ["box", "label"]:
                        move(f"{name}.{part}")
                        expected = values()
                        # X11 buttons 4/5 rotate vertically; 6/7 rotate horizontally.
                        for button in [5, 4, 7, 6]:
                            move(f"{name}.{part}")
                            before_scroll = current["scroll"]
                            click(button)
                            assert values() == expected, f"wheel {button} activated {name}.{part}: {current}"
                            if name == "controls.checkbox" and button == 5:
                                assert current["scroll"] > before_scroll, "wheel did not reach native scrolling"
                        for button in [2, 3, 8, 9]:
                            move(f"{name}.{part}")
                            click(button)
                            assert values() == expected, f"non-primary button {button} activated {name}.{part}: {current}"
                    move(f"{name}.box")
                    before = values()
                    click(1)
                    after = values()
                    index = 0 if name == "checkbox.enabled" else 1
                    assert after[index] != before[index] and after[1 - index] == before[1 - index], \
                        f"primary click failed to toggle only {name}: {current}"
            if os.environ.get("OPERAD_NATIVE_POINTER_SCREENSHOT"):
                subprocess.run(["import", "-window", window, os.environ["OPERAD_NATIVE_POINTER_SCREENSHOT"]],
                               env=env, check=True)
            print("Native pointer probe passed: checkbox box/label, checked/unchecked, four wheel directions, "
                  "middle/right/back/forward buttons, primary clicks, sidebar scrolling.")
        finally:
            stop(app)
            reader.join(timeout=5)


def main():
    xvfb = os.environ.get("XVFB_BIN") or shutil.which("Xvfb")
    if not xvfb or not shutil.which("xdotool"):
        raise SystemExit("Native pointer probe requires Xvfb (or XVFB_BIN) and xdotool")
    root = pathlib.Path(__file__).resolve().parent.parent
    binary = pathlib.Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else root / "target/debug/examples/native_pointer"
    with tempfile.TemporaryDirectory(prefix="operad-native-pointer-") as temporary:
        directory = pathlib.Path(temporary)
        with open(directory / "display.log", "w") as log:
            read_fd, write_fd = os.pipe()
            display = subprocess.Popen([xvfb, "-displayfd", str(write_fd), "-screen", "0", "1024x800x24",
                                        "-dpi", "96", "-ac", "-nolisten", "tcp"],
                                       pass_fds=(write_fd,), stdout=log, stderr=log)
            os.close(write_fd)
            try:
                if not select.select([read_fd], [], [], 10)[0]:
                    raise AssertionError("Xvfb did not publish a display")
                number = os.read(read_fd, 64).decode().strip()
                if not number.isdecimal():
                    raise AssertionError("Xvfb exited before creating its display")
                env = {**os.environ, "DISPLAY": f":{number}", "WINIT_X11_SCALE_FACTOR": "1",
                       "LIBGL_ALWAYS_SOFTWARE": "1", "GALLIUM_DRIVER": "llvmpipe", "LP_NUM_THREADS": "1"}
                for name in ["WAYLAND_DISPLAY", "WAYLAND_SOCKET", "XAUTHORITY"]:
                    env.pop(name, None)
                drivers = list(pathlib.Path("/usr/share/vulkan/icd.d").glob("lvp_icd*.json"))
                if drivers:
                    env["VK_DRIVER_FILES"] = env["VK_ICD_FILENAMES"] = str(drivers[0])
                probe(binary, env, directory)
            except Exception:
                for name in ["app.log", "display.log"]:
                    path = directory / name
                    if path.exists():
                        print(path.read_text()[-8000:], file=sys.stderr)
                raise
            finally:
                os.close(read_fd)
                stop(display)


if __name__ == "__main__":
    main()
