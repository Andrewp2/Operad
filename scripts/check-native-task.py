#!/usr/bin/env python3
"""Check task completion on a real, otherwise idle native event loop.

Build with cargo build --example runtime_task, then run this script on a desktop
or under a virtual display. No keyboard/mouse events or tick actions are sent.
"""

import pathlib
import queue
import subprocess
import sys
import threading
import time


def check(binary):
    process = subprocess.Popen(
        [str(binary)], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
        text=True, bufsize=1,
    )
    lines = queue.Queue()

    def read_output():
        for line in process.stdout:
            lines.put(line.strip())
        lines.put(None)

    threading.Thread(target=read_output, daemon=True).start()
    frame = 0

    def next_frame(timeout):
        nonlocal frame
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            line = lines.get(timeout=max(0.001, deadline - time.monotonic()))
            if line is None:
                raise AssertionError(f"native probe exited: {process.poll()}")
            if line.startswith("FRAME "):
                _, number, value = line.split(" ", 2)
                assert int(number) > frame, line
                frame = int(number)
                return value
        raise queue.Empty

    def assert_idle():
        # Let startup/presentation events settle, then require a quiet interval.
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            try:
                next_frame(0.3)
            except queue.Empty:
                break
        else:
            raise AssertionError("native probe never became idle")
        try:
            next_frame(0.3)
        except queue.Empty:
            return
        raise AssertionError("native probe kept rendering while idle")

    try:
        assert next_frame(30) == "waiting"
        assert_idle()
        for command, expected in [("complete", "loaded"), ("fail", "read failed")]:
            process.stdin.write(command + "\n")
            process.stdin.flush()
            assert next_frame(10) == expected, f"{command} did not update the rendered view"
            assert_idle()
        print(f"Native task probe passed: idle, worker success/failure, UI-thread delivery ({frame} frames)")
    finally:
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()


if __name__ == "__main__":
    root = pathlib.Path(__file__).resolve().parent.parent
    check(pathlib.Path(sys.argv[1]).resolve() if len(sys.argv) > 1
          else root / "target/debug/examples/runtime_task")
