#!/usr/bin/env python3
"""Exercise the real binary over a PTY; uses only Python's standard library."""
import errno
import fcntl
import json
import os
from pathlib import Path
import pty
import select
import signal
import struct
import subprocess
import sys
import termios
import tempfile
import time


def main():
    binary = str(Path(sys.argv[1] if len(sys.argv) > 1 else "target/release/astrarium").resolve())
    with tempfile.TemporaryDirectory(prefix="astrarium-i18n-", dir="/tmp/opencode" if Path("/tmp/opencode").is_dir() else None) as config:
        environment = {**os.environ, "TERM": "xterm-256color", "XDG_CONFIG_HOME": config}
        prefs = Path(config) / "astrarium/preferences.json"
        def check(*args):
            return subprocess.check_output([binary, "--check", *args], env=environment).decode()
        assert "\nMoon:" in check(), "fresh installation did not default to English"
        exercise(binary, "en", environment)
        assert json.loads(prefs.read_text())["language"] == "es"
        assert "\nLuna:" in check(), "saved Spanish preference was not loaded"
        assert "\nMoon:" in check("--lang", "en"), "CLI did not override saved preference"
        assert json.loads(prefs.read_text())["language"] == "es", "CLI override unexpectedly persisted"
        exercise(binary, "es", environment)
        assert "\nMoon:" in check(), "saved English preference was not loaded"
        assert "\nLuna:" in check("--lang", "es"), "Spanish CLI option failed"
        prefs.write_text('{"language":"unsupported"}')
        assert "\nMoon:" in check(), "invalid preference did not fall back to English"
        bad = subprocess.run([binary, "--lang", "fr", "--check"], env=environment, capture_output=True)
        assert bad.returncode != 0, "unsupported language was accepted"
        print("PASS: English default, Spanish, F2 persistence, CLI precedence and invalid preference fallback")


def exercise(binary, language, environment):
    spanish = language == "es"
    master, slave = pty.openpty()
    before = termios.tcgetattr(slave)
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 140, 0, 0))
    process = subprocess.Popen([
        binary, "--lat=0", "--lon=0", "--location=PTY test", "--at=2026-01-01T00:00:00Z",
    ], stdin=slave, stdout=slave, stderr=slave, env=environment, start_new_session=True)
    output = bytearray()

    def drain(duration=0.20):
        end = time.monotonic() + duration
        chunk = bytearray()
        while time.monotonic() < end:
            if select.select([master], [], [], max(0, end - time.monotonic()))[0]:
                try:
                    data = os.read(master, 65536)
                except OSError as error:
                    if error.errno == errno.EIO:
                        break
                    raise
                if not data:
                    break
                chunk.extend(data)
        output.extend(chunk)
        return chunk.decode(errors="replace")

    def send(value):
        os.write(master, value)
        return drain()

    try:
        drain(0.8)
        assert b"\x1b[?1049h" in output, "alternate screen not enabled"
        assert b"\x1b[?1006h" in output, "SGR mouse capture not enabled"
        assert ("CIELO LOCAL" if spanish else "LOCAL SKY") in output.decode(), "incorrect startup language"
        send(b"/Sirius")
        found = send(b"\r")
        assert "Sirius" in found, "search did not select Sirius"
        send(b"/Jupiter")
        assert ("Júpiter" if spanish else "Jupiter") in send(b"\r"), "unaccented planet search failed"
        send(b"/Luna")
        assert ("Iluminada" if spanish else "Illuminated") in send(b"\r"), "Moon phase information missing"
        assert send(b"]"), "time advance did not redraw solar objects"
        send(b"/Sol")
        sun = send(b"\r")
        # Ratatui emits cell diffs, so complete unchanged phrases need not be
        # present in this byte stream. Full horizon labels are tested in Rust.
        assert ("Sol" if spanish else "Sun") in sun, "Sun search did not update the selection"
        assert send(b"h"), "full-sky toggle did not redraw"
        assert send(b"\r"), "could not center Sun below the horizon in full-sky mode"
        send(b"h")
        send(b"\x1b[<0;45;18M")
        moved = send(b"\x1b[<32;55;20M")
        send(b"\x1b[<0;55;20m")
        assert moved, "drag did not redraw"
        assert send(b"\x1b[<64;55;20M"), "wheel did not redraw"
        assert ("GUÍA" if spanish else "GUIDE") in send(b"?"), "help did not open"
        send(b"\x1b")
        send(b"l")
        send(b"\x15Bad;NaN;0")
        assert ("Latitud" if spanish else "Latitude") in send(b"\r"), "invalid location was not rejected"
        send(b"\x1b")
        for cols, rows in [(60, 20), (20, 8), (140, 40)]:
            fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))
            os.kill(process.pid, signal.SIGWINCH)
            drain()
            assert process.poll() is None, "process exited during resize"
        # Switch while a search is open; its query and focus must survive.
        send(b"/Luna")
        send(b"\x1bOQ")  # F2 (SS3)
        prefs = Path(environment["XDG_CONFIG_HOME"]) / "astrarium/preferences.json"
        assert json.loads(prefs.read_text())["language"] == ("en" if spanish else "es")
        assert ("Moon" if spanish else "Luna") in send(b"\r"), "language switch lost search input or selection"
        send(b"q")
        assert process.wait(timeout=5) == 0, output.decode(errors="replace")[-2000:]
        drain()
        assert b"\x1b[?1049l" in output, "alternate screen not restored"
        assert b"\x1b[?1006l" in output, "mouse not restored"
        assert termios.tcgetattr(slave) == before, "terminal attributes not restored"
        assert b"panicked" not in output
        print(f"PASS ({language}): PTY startup, search, lunar phase, time, full sky, mouse, zoom, help, validation, resize, F2, clean exit")
    finally:
        if process.poll() is None:
            process.kill()
            process.wait()
        os.close(master)
        os.close(slave)


if __name__ == "__main__":
    main()
