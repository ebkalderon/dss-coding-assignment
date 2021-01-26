# Disney Streaming Services (DSS) Main Menu

Main menu for a DSS-inspired streaming media application, written in Rust.

![Screenshot](./screenshot.png)

## Getting started

To build this project on Windows, macOS, or Linux, you will need a recent
version of the [Rust toolchain](https://www.rust-lang.org/) at least version
1.46.0, at minimum, but versions 1.48.0 and newer are strongly recommended
because they perform better with async code ([rust-lang/rust#78410]).

[rust-lang/rust#78410]: https://github.com/rust-lang/rust/pull/78410

If [`rustup`](https://rustup.rs/) is available on your system, the included 
[`rust-toolchain`](./rust-toolchain) file at the root directory of this
repository should automatically fetch and install the Rust toolchain for you, if
not already present on your system, as soon as you execute any `cargo` command
in the shell.

The following external dependencies will also be required on your system:

* [SDL2], for system window and render context management, input handling, and
  hardware-accelerated rendering.
* [SDL2_image], for loading arbitrary image files as SDL textures.
* [SDL2_ttf], for loading and rendering TrueType fonts as SDL textures.

[SDL2]: https://www.libsdl.org/
[SDL2_image]: https://www.libsdl.org/projects/SDL_image/
[SDL2_ttf]: https://www.libsdl.org/projects/SDL_ttf/

### Windows (MSVC with vcpkg)

```bat
vcpkg.exe install sdl2:x64-windows sdl2-image:x64-windows sdl2-ttf:x64-windows
```

### macOS

```bash
brew install sdl2 sdl2_image sdl2_ttf
```

### Linux

```bash
# Ubuntu/Debian
sudo apt install libsdl2 libsdl2-image libsdl2-ttf

# Arch Linux
sudo pacman -Sy sdl2 sdl2_image sdl2_ttf
```

## Compiling

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

## Usage

The input controls for navigating the UI are listed below:

Action            | Controls
------------------|-------------------------------------------------------------
Navigate menu     | <kbd>↑</kbd>, <kbd>↓</kbd>, <kbd>←</kbd>, <kbd>→</kbd>
Toggle fullscreen | <kbd>F11</kbd>
Close window      | <kbd>Esc</kbd> or "close" button

## Credits

Includes the [Cocogoose Pro] TrueType font family, which is free for personal
use.

[Cocogoose Pro]: https://www.1001fonts.com/cocogoose-pro-font.html
