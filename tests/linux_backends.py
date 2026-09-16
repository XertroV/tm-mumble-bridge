"""Release-binary integration across X11, Wayland, missing libraries and no display.

Invoke under xvfb-run. Weston supplies the independent native Wayland compositor.
"""
import os
from pathlib import Path
import socket
import subprocess
import sys
import tempfile
import time
from linux_smoke import main as smoke


def main(directory):
    gui = str(Path(directory).resolve() / 'tm-mumble-link')
    tui = str(Path(directory).resolve() / 'tm-mumble-link-tui')
    base = {k: v for k, v in os.environ.items()
            if k not in ('WAYLAND_DISPLAY', 'WAYLAND_SOCKET', 'TM_MUMBLE_BACKEND')}
    smoke(gui, dict(base, TM_MUMBLE_BACKEND='x11'))
    smoke(gui, dict(base, WAYLAND_DISPLAY='/nonexistent/tm-mumble-test'))
    headless = {k: v for k, v in base.items() if k != 'DISPLAY'}
    failed = subprocess.run([gui], env=headless, capture_output=True, timeout=15)
    assert failed.returncode == 1, failed.stderr
    assert b'tm-mumble-link-tui' in failed.stderr and b'panicked' not in failed.stderr
    smoke(tui, headless)
    with tempfile.TemporaryDirectory() as directory, tempfile.TemporaryFile() as log:
        runtime = Path(directory)
        env = dict(headless, XDG_RUNTIME_DIR=str(runtime), WAYLAND_DISPLAY='test-wayland')
        compositor = subprocess.Popen([
            'weston', '--backend=headless-backend.so', '--renderer=pixman',
            '--socket=test-wayland', '--idle-time=0', '--no-config',
        ], env=env, stdout=log, stderr=log)
        try:
            deadline = time.monotonic() + 15
            while not (runtime / 'test-wayland').exists():
                if compositor.poll() is not None or time.monotonic() > deadline:
                    raise AssertionError('Weston failed to start')
                time.sleep(0.05)
            smoke(gui, dict(env, TM_MUMBLE_BACKEND='wayland'))
            smoke(gui, env)
            # A pre-opened Wayland socket must survive backend selection.
            with socket.socket(socket.AF_UNIX) as supplied:
                supplied.connect(str(runtime / 'test-wayland'))
                supplied_env = {k: v for k, v in env.items() if k != 'WAYLAND_DISPLAY'}
                supplied_env['WAYLAND_SOCKET'] = str(supplied.fileno())
                smoke(gui, supplied_env, (supplied.fileno(),))
            shim = runtime / 'deny-wayland.so'
            subprocess.run(['cc', '-shared', '-fPIC', 'tests/deny_wayland.c',
                            '-o', str(shim), '-ldl'], check=True)
            # The socket exists, but dlopen fails exactly as in the user report.
            smoke(gui, dict(env, DISPLAY=base['DISPLAY'], LD_PRELOAD=str(shim)))
        finally:
            compositor.terminate()
            try:
                compositor.wait(timeout=5)
            except subprocess.TimeoutExpired:
                compositor.kill()
                compositor.wait()
            log.seek(0)
            print(log.read().decode(errors='replace'))
    print('PASS: X11, native Wayland, supplied socket, NoWaylandLib fallback, headless CLI')


if __name__ == '__main__':
    main(sys.argv[1])
