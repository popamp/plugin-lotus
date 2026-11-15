# Lotus Bloom for Popamp

![Lotus Bloom: rings of teal, violet, rust and plum petals around a golden heart](screenshot.png)

A [Popamp](https://github.com/popamp/popamp) visualizer plug-in: a
many-petalled flower whose rings of petals swell and shrink with the
music. The outer rings follow the bass, the inner ones the treble, and
each kick of the drum sends a wave out from the heart.

`src/pulse.rs` listens: once a frame it turns the spectrum into a level
per ring and finds the kicks, and hands them to the shader as the scene's
data. `src/flower.wgsl` draws. The plug-in says what to draw and the
player makes every GPU call, so it needs no permissions.

## Install

You need Popamp Plus, or any build with visualizers
(`cargo build --release --features visualizers`).

**From the Gallery** (Popamp 0.8.0 or newer, Linux and Windows): in
Popamp open Options > Gallery, find Lotus Bloom and install it. Updates
come the same way.

**As a package** (Popamp 0.7.0 or newer, Linux), from
[apt.popamp.app](https://apt.popamp.app) on Debian and Ubuntu or from the
[openSUSE and Fedora repository](https://github.com/popamp/popamp/blob/main/docs/building.md#opensuse-and-fedora):

```sh
sudo apt install popamp-plugin-lotus      # Debian, Ubuntu
sudo zypper install popamp-plugin-lotus   # openSUSE
sudo dnf install popamp-plugin-lotus      # Fedora
```

**From this repository**: clone it into Popamp's plug-ins folder, where
a copy of the same name wins over the package:

```sh
git clone https://github.com/popamp/plugin-lotus ~/.local/share/popamp/plugins/lotus
```

The folder must be called `lotus`, the plug-in's id; updating is
`git pull` there.

However it came, turn it on in Options > Plug-ins…: tick Lotus Bloom
and press OK. It opens from the Visualization menu. F toggles full
screen, Esc leaves it or closes the window.

## Build

```sh
rustup target add wasm32-unknown-unknown   # once
./build.sh                                 # writes plugin.wasm
cargo test                                 # checks the pulse and draws the real scene
```

The drawing tests use the player's own renderer; a machine with no GPU
skips them. To look at the flower at rest and in full swell:

```sh
LOTUS_PREVIEW=/tmp cargo test --test drawing -- --ignored preview
```

`plugin.wasm` is committed, so cloning the repository installs a working
plug-in; rebuild it with `./build.sh` whenever `src/` changes.

## License

MIT or Apache-2.0, at your option.
