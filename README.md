# Disney Streaming Services (DSS) Main Menu

## Getting started

### Prerequisites

To build this project on Windows, macOS, or Linux, you will need a recent stable
version of the [Rust toolchain](https://www.rust-lang.org/) (this project was
tested with 1.49.0).

If [`rustup`](https://rustup.rs/) is available on your system, the included 
[`rust-toolchain`](./rust-toolchain) file at the root directory of this
repository should automatically fetch and install Rust 1.49.0 for you, if it's
not already present on your system, as soon as you execute any `cargo` command
in the shell.

The following external dependencies will also be required on your system:

* [SDL2](https://www.libsdl.org/)
* `SDL2_Image`
* `SDL2_TTF`

#### Windows (MSVC with vcpkg)

```bat
vcpkg.exe install sdl2:x64-windows sdl2-image:x64-windows sdl2-ttf:x64-windows
```

#### macOS

```bash
brew install sdl2 sdl2_image sdl2_ttf
```

#### Linux

```bash
# Ubuntu/Debian
sudo apt install -y libsdl2 libsdl2-image libsdl2-ttf

# Arch Linux
sudo pacman -Sy sdl2 sdl2_image sdl2_ttf
```

### Compiling

To compile the application in release mode and start it, simply run this command
in your terminal:

```bash
cargo run --release
```

To execute the included unit test suite, run:

```bash
cargo test
```

To generate HTML documentation for the public crate API, run:

```bash
cargo doc --open
```
