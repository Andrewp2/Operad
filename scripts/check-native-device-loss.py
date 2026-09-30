#!/usr/bin/env python3
"""Destroy a real GPU device while the native host is idle and require clean exit.

Build --example native_device_loss and run under an isolated Xvfb display.
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
    try:
        frame = 0
        deadline = time.monotonic() + 30
        while time.monotonic() < deadline:
            try:
                line = lines.get(timeout=0.3 if frame else 10)
            except queue.Empty:
                if frame:
                    break
                raise AssertionError("native device-loss fixture did not render")
            assert line is not None, f"fixture exited before loss: {process.poll()}"
            assert line.startswith("FRAME "), line
            frame = int(line.split()[1])
        else:
            raise AssertionError("native host did not become idle")

        process.stdin.write("lose\n")
        process.stdin.flush()
        destroyed = False
        handled = False
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            try:
                line = lines.get(timeout=max(0.001, deadline - time.monotonic()))
            except queue.Empty:
                raise AssertionError("lost device did not wake and stop the idle native host")
            if line is None:
                break
            if line == "DEVICE_DESTROYED":
                destroyed = True
            elif line.startswith("DEVICE_LOSS_HANDLED "):
                handled = True
                assert int(line.split()[1]) == frame, "host rendered another frame after loss"
            else:
                raise AssertionError(f"unexpected work after device loss: {line}")
        assert destroyed and handled, f"missing device loss or shutdown: {destroyed=}, {handled=}"
        assert process.wait(timeout=5) == 0, "device-loss fixture did not exit successfully"
        print("Native device loss probe passed: idle wakeup, classified failure, no additional frame")
    finally:
        if process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()


if __name__ == "__main__":
    root = pathlib.Path(__file__).resolve().parent.parent
    binary = (pathlib.Path(sys.argv[1]).resolve() if len(sys.argv) > 1
              else root / "target/debug/examples/native_device_loss")
    check(binary)
    result = subprocess.run([str(binary), "during-render"], text=True,
                            stdout=subprocess.PIPE, check=True, timeout=30)
    assert result.stdout.splitlines() == [
        "FRAME 1", "DEVICE_DESTROYED", "DEVICE_LOSS_HANDLED 1",
    ], f"unexpected rendering after canvas destroyed the device: {result.stdout}"
    print("Native device loss probe passed: canvas callback failure, no retry or GPU use afterward")
