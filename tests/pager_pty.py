"""Unix pager regressions using only Python's standard library.

Run after cargo build/test: python3 tests/pager_pty.py target/debug/mat
"""
import codecs
import fcntl
import os
from pathlib import Path
import pty
import re
import select
import signal
import struct
import subprocess
import sys
import tempfile
import termios
import time
import unicodedata
import unittest

BINARY = str(Path(sys.argv.pop(1)).resolve())
CSI = re.compile(r"\x1b\[([0-?]*)([ -/]*)([@-~])")


class Screen:
    """Interpret the cursor, erase, and style commands emitted by Ratatui."""

    def __init__(self, width, height):
        self.width, self.height = width, height
        self.cells = [[" "] * width for _ in range(height)]
        self.x = self.y = 0
        self.pending = ""
        self.styles = []

    def feed(self, text):
        self.pending += text
        while self.pending:
            if self.pending.startswith("\x1b"):
                match = CSI.match(self.pending)
                if match is None:
                    if len(self.pending) < 2 or self.pending.startswith("\x1b["):
                        return  # Incomplete command; continue on the next read.
                    raise AssertionError(f"Unexpected escape: {self.pending[:40]!r}")
                raw, _, command = match.groups()
                self.pending = self.pending[match.end():]
                values = [int(value or 0) for value in raw.lstrip("?").split(";")]
                first = values[0]
                if command in "Hf":
                    self.y = (first or 1) - 1
                    self.x = ((values[1] if len(values) > 1 else 1) or 1) - 1
                elif command == "G":
                    self.x = (first or 1) - 1
                elif command == "d":
                    self.y = (first or 1) - 1
                elif command in "ABCD":
                    amount = first or 1
                    if command == "A": self.y -= amount
                    if command == "B": self.y += amount
                    if command == "C": self.x += amount
                    if command == "D": self.x -= amount
                elif command == "J" and first in (2, 3):
                    self.cells = [[" "] * self.width for _ in range(self.height)]
                elif command == "K" and 0 <= self.y < self.height:
                    start = 0 if first == 2 else self.x
                    self.cells[self.y][start:] = [" "] * (self.width - start)
                elif command == "m":
                    self.styles.extend(values)
                continue
            character, self.pending = self.pending[0], self.pending[1:]
            if character == "\r":
                self.x = 0
            elif character == "\n":
                self.y += 1
            elif not unicodedata.combining(character):
                if 0 <= self.x < self.width and 0 <= self.y < self.height:
                    self.cells[self.y][self.x] = character
                self.x += 2 if unicodedata.east_asian_width(character) in "WF" else 1

    def text(self):
        return "\n".join("".join(row) for row in self.cells)


def controlling_terminal():
    fcntl.ioctl(0, termios.TIOCSCTTY, 0)


class Pager:
    def __init__(self, path, args=(), width=100, height=5, no_color=False):
        self.closed = False
        self.master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", height, width, 0, 0))
        environment = os.environ.copy()
        environment["TERM"] = "xterm-256color"
        environment.pop("NO_COLOR", None)
        if no_color:
            environment["NO_COLOR"] = "1"
        self.screen = Screen(width, height)
        self.decoder = codecs.getincrementaldecoder("utf-8")("replace")
        self.output = ""
        try:
            self.process = subprocess.Popen(
                [BINARY, "-t", "dark", *args, str(path)],
                stdin=slave, stdout=slave, stderr=slave, env=environment,
                start_new_session=True, preexec_fn=controlling_terminal,
            )
        finally:
            os.close(slave)

    def pump(self):
        if select.select([self.master], [], [], 0.05)[0]:
            try:
                data = os.read(self.master, 65536)
            except OSError:
                data = b""
            text = self.decoder.decode(data)
            self.output += text
            self.screen.feed(text)

    def wait(self, predicate, description):
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            self.pump()
            if predicate():
                return
            if self.process.poll() is not None:
                break
        raise AssertionError(f"{description}\nScreen:\n{self.screen.text()}\nOutput tail: {self.output[-1000:]!r}")

    def expect(self, text):
        self.wait(lambda: text in self.screen.text(), f"Expected {text!r}")

    def send(self, keys):
        os.write(self.master, keys.encode())

    def resize(self, width, height):
        self.screen = Screen(width, height)
        fcntl.ioctl(self.master, termios.TIOCSWINSZ, struct.pack("HHHH", height, width, 0, 0))
        os.kill(self.process.pid, signal.SIGWINCH)

    def close(self):
        if self.closed:
            return
        self.closed = True
        try:
            if self.process.poll() is None:
                self.send("q")
                try:
                    self.process.wait(timeout=2)
                except subprocess.TimeoutExpired:
                    self.process.kill()
                    self.process.wait()
            assert self.process.returncode == 0, self.output[-2000:]
        finally:
            os.close(self.master)


class PagerTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.path = Path(self.directory.name) / "log.txt"

    def pager(self, content, args=(), **kwargs):
        self.path.write_text(content)
        pager = Pager(self.path, args, **kwargs)
        self.addCleanup(pager.close)
        return pager

    def test_wrapping_gutter_toggle_resize_and_reload(self):
        pager = self.pager("x" * 1000 + "END\n", ("--wrap", "wrap", "-n"), width=20)
        pager.expect("x" * 17)
        pager.send("G")
        pager.expect("END")
        pager.send("#")
        pager.expect("x" * 20)
        pager.resize(100, 5)
        pager.expect("END")
        pager.send("R")
        pager.expect("END")
        self.assertIsNone(pager.process.poll())

    def test_follow_starts_at_end_and_tracks_append_and_rotation(self):
        pager = self.pager("old\n" * 49 + "LAST_ORIGINAL_ROW\n", ("--follow",))
        pager.expect("LAST_ORIGINAL_ROW")
        pager.expect("[FOLLOW]")
        with self.path.open("a") as output:
            output.write("NEW_APPENDED_CONTENT\n")
        pager.expect("NEW_APPENDED_CONTENT")
        self.path.rename(self.path.with_suffix(".old"))
        self.path.write_text("ROTATED_CONTENT\n")
        pager.expect("ROTATED_CONTENT")

    def test_search_color_policy_and_manual_reload(self):
        for mode, no_color, colored in [("never", False, False), ("auto", True, False), ("always", True, True)]:
            with self.subTest(mode=mode):
                pager = self.pager("hello\n", ("--color", mode, "-s", "hello"), no_color=no_color)
                pager.expect("hello")
                pager.expect("Match 0/1")
                pager.send("/hello\r")
                pager.expect("Match 0/1")
                self.path.write_text("hello reloaded\n")
                pager.send("R")
                pager.expect("hello reloaded")
                if colored:
                    self.assertTrue(38 in pager.screen.styles or 48 in pager.screen.styles)
                else:
                    self.assertTrue(set(pager.screen.styles) <= {0, 39, 49, 59}, pager.screen.styles)
                pager.close()

    def test_reload_failure_keeps_content_and_recovers(self):
        pager = self.pager("LAST_GOOD_CONTENT\n")
        pager.expect("LAST_GOOD_CONTENT")
        self.path.write_bytes(b"\0binary")
        pager.send("R")
        pager.expect("Reload failed:")
        pager.expect("LAST_GOOD_CONTENT")
        self.path.write_text("RECOVERED_CONTENT\n")
        pager.send("R")
        pager.expect("RECOVERED_CONTENT")
        self.assertNotIn("Reload failed:", pager.screen.text())


if __name__ == "__main__":
    unittest.main()
