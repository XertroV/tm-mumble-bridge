"""Exercise the real Linux bridge against a Mumble Link shared-memory fixture.

Run under an X11 display (e.g. xvfb-run -a). Uses only Python's standard library.
Refuses to overwrite an existing Mumble mapping or use an occupied bridge port.
"""

import ctypes as c
import json
import mmap
import os
from pathlib import Path
import socket
import struct
import subprocess
import sys
import tempfile
import time


# Independent C ABI definition from Mumble's plugins/link/LinkedMem.h.
class LinkedMem(c.Structure):
    _fields_ = [
        ("version", c.c_uint32), ("tick", c.c_uint32),
        ("avatar_pos", c.c_float * 3), ("avatar_front", c.c_float * 3),
        ("avatar_top", c.c_float * 3), ("name", c.c_wchar * 256),
        ("camera_pos", c.c_float * 3), ("camera_front", c.c_float * 3),
        ("camera_top", c.c_float * 3), ("identity", c.c_wchar * 256),
        ("context_len", c.c_uint32), ("context", c.c_ubyte * 256),
        ("description", c.c_wchar * 2048),
    ]


def send(sock, message):
    data = message if isinstance(message, bytes) else json.dumps(message).encode()
    # message-io FramedTcp prefixes each message with an unsigned varint length.
    size = len(data)
    prefix = bytearray()
    while size >= 128:
        prefix.append((size & 127) | 128)
        size >>= 7
    prefix.append(size)
    sock.sendall(prefix + data)


def wait_for(check, process, what):
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise AssertionError(f"Bridge exited with {process.returncode}: {what}")
        value = check()
        if value:
            return value
        time.sleep(0.05)
    raise AssertionError(f"Timed out: {what}")


def main(binary, env=None, pass_fds=()):
    assert sys.platform == "linux"
    assert c.sizeof(c.c_wchar) == 4
    assert c.sizeof(LinkedMem) == 10580
    # Check the port before starting a process that must own this listener.
    with socket.socket() as probe:
        probe.bind(("127.0.0.1", 46323))

    path = Path(f"/dev/shm/MumbleLink.{os.getuid()}")
    fd = os.open(path, os.O_CREAT | os.O_EXCL | os.O_RDWR, 0o600)
    process = None
    try:
        os.ftruncate(fd, c.sizeof(LinkedMem))
        with mmap.mmap(fd, c.sizeof(LinkedMem)) as memory, tempfile.TemporaryFile() as log:
            process = subprocess.Popen([binary], stdout=log, stderr=log, env=env, pass_fds=pass_fds)
            try:
                def connect():
                    try:
                        return socket.create_connection(("127.0.0.1", 46323), timeout=0.2)
                    except OSError:
                        return None

                def snapshot():
                    return LinkedMem.from_buffer_copy(memory)

                with wait_for(connect, process, "client starts the TCP listener") as sock:
                    # Non-ASCII text distinguishes Linux wchar_t from Windows UTF-16.
                    send(sock, {"PlayerDetails": ["Racer 🏁é", "login"]})
                    send(sock, {"ServerDetails": ["test-server", "All"]})
                    player = {"pos": [1.25, 2.5, -3.75], "dir": [0, 0, 1], "up": [0, 1, 0]}
                    camera = {"pos": [-4, 5, 6], "dir": [1, 0, 0], "up": [0, 1, 0]}
                    send(sock, {"Positions": {"p": player, "c": camera}})
                    wait_for(lambda: snapshot().tick > 0, process, "JSON position update")
                    state = snapshot()
                    assert state.version == 2
                    assert state.name == "TM-Proximity-Chat"
                    assert state.description == "Bridge to TM2020 plugin for proximity chat"
                    assert state.identity.startswith("Racer 🏁é|login|")
                    assert bytes(state.context[:state.context_len]) == b"TM|test-server|All"
                    for field, expected in [
                        ("avatar_pos", player["pos"]), ("avatar_front", player["dir"]),
                        ("avatar_top", player["up"]), ("camera_pos", camera["pos"]),
                        ("camera_front", camera["dir"]), ("camera_top", camera["up"]),
                    ]:
                        assert list(getattr(state, field)) == expected, field

                    tick = state.tick
                    values = [7, 8, 9, 0, 0, 1, 0, 1, 0, -7, -8, -9, 1, 0, 0, 0, 1, 0]
                    # Reject a truncated binary position without killing the bridge.
                    send(sock, b"\x01\x00")
                    send(sock, b"\x01" + struct.pack("<18f", *values))
                    wait_for(lambda: snapshot().tick > tick, process, "binary position update")
                    state = snapshot()
                    assert list(state.avatar_pos) == values[:3]
                    assert list(state.camera_pos) == values[9:12]

                    tick = state.tick
                    send(sock, {"LeftServer": []})
                    send(sock, {"Positions": {"p": player, "c": camera}})
                    wait_for(lambda: snapshot().tick > tick, process, "leave-server context update")
                    state = snapshot()
                    assert bytes(state.context[:state.context_len]) == b"TM||All"
                print("PASS: native client startup, framed TCP (JSON/binary), Linux Mumble ABI, Unicode, context")
            finally:
                process.terminate()
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()
                log.seek(0)
                print(log.read().decode(errors="replace"))
    finally:
        os.close(fd)
        path.unlink()


if __name__ == "__main__":
    main(sys.argv[1])
