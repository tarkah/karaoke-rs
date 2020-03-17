<p align="center">
  <a href="assets/logo.png">
    <img src="assets/logo.png" width="30%" />
  </a>
</p>
<p align="center">
  A simple, network enabled karaoke player in Rust.
</p>
<p align="center">
  <a href="https://dev.azure.com/tarkah/karaoke-rs/_build/latest?definitionId=1&branchName=master">
    <img src="https://dev.azure.com/tarkah/karaoke-rs/_apis/build/status/tarkah.karaoke-rs?branchName=master" />
  </a>
</p>

---

Your karaoke collection can be browsed and queued from a self served website and played either natively on your computer, or remotely through any browser. Only supports MP3+G (mp3 & corresponding cdg) files.

**_Now includes a web player. Songs can be played from any modern browser, anywhere in the world! Use command line flag `--use-web-player` to enable this feature. Player is accessible from the `/player` page on the website and can be controlled just like the native player through commands on the queue page._**

- [Setup](#setup)
  - [Linux](#linux)
  - [Windows](#windows)
  - [Build from Source](#build-from-source)
- [CLI Arguments](#cli-arguments)
- [Screenshots](#screenshots)
  - [Songs Page](#songs-page)
  - [Artists Page](#artists-page)
  - [Queue Page](#queue-page)
  - [Player](#player)
- [Acknowledgments](#acknowledgments)

## Setup
### Linux
- Download latest release binary or build from source
- Run `karaoke-rs --help` to see all arguments
- Place your song collection at `~/.local/share/karaoke-rs/songs`, or specify location via `--songs path/to/song/directory`
- Default configuration file is created at `~/.config/karaoke-rs/config.yaml`. This can be copied / changed and specified via `--config path/to/config.yaml`
- You may need to force disable vsync to eliminate flickering, set environment variable `vblank_mode=0`

### Windows
- Download latest release binary or build from source
- Double click `karaoke-rs.exe` to run with default configuration. Run from command prompt / powershell `karaoke-rs.exe --help` to see all arguments
- Place your song collection at `%APPDATA%\karaoke-rs\songs`, or specify location via `--songs C:\path\to\song\directory`
- Default configuration file is created at `%APPDATA%\karaoke-rs\config.yaml`. This can be copied / changed and specified via `--config C:\path\to\config.yaml`
- Ensure all paths supplied via argument are absolute from the root of the applicable drive. Relative paths appear to cause program to crash

### Build from Source
- Build frontend

First [install wasm-pack](https://rustwasm.github.io/wasm-pack/installer/), then run:
```sh
cd frontend
npm install
npm run build
```
- Compile
```sh
cd ..
cargo build --release
```
- Binary located at `target/release/karaoke-rs`

## CLI Arguments
```
karoake-rs 0.11.0
tarkah <admin@tarkah.dev>
A simple, network enabled karaoke player in Rust

USAGE:
    karaoke-rs [FLAGS] [OPTIONS]

FLAGS:
    -h, --help              Prints help information
    -w, --use-web-player    Use web player instead of native player
    -V, --version           Prints version information

OPTIONS:
    -c, --config <FILE>                Sets a custom config file
    -d, --data <DIR>                   Sets a custom data directory
    -p, --port <PORT>                  Specify website port
        --port-ws <PORT_WS>            Specify a websocket port when using the web player feature
    -r, --refresh-collection <BOOL>    Specify if collection should be refreshed on startup [possible values: true,
                                       false]
    -s, --songs <DIR>                  Sets a custom song directory
```

## Screenshots

### Songs Page
![songs](/screenshots/songs.png?raw=true)

### Artists Page
![artists](/screenshots/artists.png?raw=true)

### Queue Page
![queue](/screenshots/queue.png?raw=true)

### Player
![player1](/assets/background.png?raw=true)

![player1](/screenshots/player_1.png?raw=true)

![player2](/screenshots/player_2.png?raw=true)


## Acknowledgments

- [@maxjoehnk](https://github.com/maxjoehnk) - Thanks for designing the frontend!
- [@Keavon](https://github.com/Keavon) - Thanks for helping extensively test the new frontend & web player!