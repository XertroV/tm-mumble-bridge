# TM to Mumble bridge

Connects Trackmania's proximity-chat plugin to Mumble's Link plugin. The bridge
receives game data on `127.0.0.1:46323` and writes positional audio data for Mumble.

## Linux

Run **both Mumble and this bridge natively on Linux**, as the same user.
Trackmania and its Openplanet proximity-chat plugin can run under Wine/Proton;
the plugin connects to the native bridge over localhost TCP.

1. Start Mumble. Enable the **Link** plugin and its positional-data permission
   (or **Link to Game and Transmit Position**, depending on the Mumble version).
   Enable positional audio under Audio Output as well.
2. Start `tm-mumble-link` in your desktop session (X11 or Wayland). If it was
   started before Mumble, click **Connect to Mumble** after enabling Link.
3. Start Trackmania with the proximity-chat plugin enabled. The bridge should
   show a client connection and changing player/camera positions. Once position
   updates arrive, Mumble should report that it linked to `TM-Proximity-Chat`.

Linux uses Mumble's POSIX shared memory `/MumbleLink.<uid>` (visible as
`/dev/shm/MumbleLink.<uid>`), including the native Linux wide-character layout.
The existing `mumble-link` crate handles this format. Running the Windows bridge
inside Wine will not connect it to native Linux Mumble.

The Linux bridge uses a normal window without a tray icon; minimize it using
your window manager and close it to exit. If Wayland's runtime client library
is unavailable but XWayland is present, startup automatically falls back to X11.
Use `TM_MUMBLE_BACKEND=wayland` or `TM_MUMBLE_BACKEND=x11` to select a backend
explicitly. A display-free client is available as `tm-mumble-link-tui`; it never
initializes a graphical backend and is suitable for headless machines, SSH, or
Wayland sessions without XWayland:

```sh
./tm-mumble-link-tui
```

Direct `ManiaPlanet_Telemetry` access
and the Alt-at-startup telemetry selector are Windows-only. Linux uses the TCP
plugin mode automatically.

If connection fails, check that Mumble's Link plugin is enabled, both processes
run under the same user, and both can access the same `/dev/shm`. Sandboxed
packages may isolate shared memory or localhost; use native packages sharing
these resources. After restarting Mumble, restart the bridge to reopen its
shared-memory mapping. Only run one application writing to Mumble Link at a time.

### Build

Install a current stable Rust toolchain and the native build dependencies. On
Debian/Ubuntu:

```sh
sudo apt-get install build-essential pkg-config libxkbcommon-dev libxkbcommon-x11-0 libwayland-dev
cargo build --release --locked
./target/release/tm-mumble-link
./target/release/tm-mumble-link-tui
```

The GUI uses wgpu and requires a working graphics driver (for example Vulkan).
On headless Ubuntu CI, `libvulkan1 mesa-vulkan-drivers xvfb` provide software
rendering and a virtual X11 display.

## Windows

Build with `cargo build --release --locked`. The tray icon, Alt-at-startup mode
selector, and direct ManiaPlanet telemetry mode remain available on Windows.

## Tests

```sh
cargo test --locked
cargo build --locked
# Linux only; stop Mumble and any other bridge instance before running:
xvfb-run -a python3 tests/linux_smoke.py target/debug/tm-mumble-link
```

The smoke test starts the actual GUI, connects over framed TCP, and reads a
Mumble-compatible shared-memory fixture. It covers JSON and binary positions,
truncated messages, Unicode identity, and server context changes. It refuses
to overwrite an existing Mumble mapping. CI builds/tests on Linux and Windows
and runs this integration test on Linux.

For a manual end-to-end check, use a native Linux Mumble client and Trackmania
under Proton with the plugin enabled. Confirm Mumble links, then check directional
audio with a second player in the same server/team. CI does not run Trackmania
or verify audible output. Wayland desktop behavior also needs a manual check;
the automated GUI test uses X11.

Protocol references:
[Mumble Link layout and naming](https://github.com/mumble-voip/mumble/blob/master/plugins/link/LinkedMem.h),
[mumble-link Unix implementation](https://github.com/SpaceManiac/mumble-link-rs/blob/master/src/unix.rs).

