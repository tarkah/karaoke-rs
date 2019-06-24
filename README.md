<p align="center">
  <a href="assets/logo.png">
    <img src="assets/logo.png" width="30%" />
  </a>
</p>
<p align="center">
  A simple, network enabled karaoke player in Rust.
</p>
<p align="center">
  <a href="https://dev.azure.com/cforsstrom18/karaoke-rs/_build/latest?definitionId=1&branchName=master">
    <img src="https://dev.azure.com/cforsstrom18/karaoke-rs/_apis/build/status/tarkah.karaoke-rs?branchName=master" />
  </a>
</p>

---

Your karaoke collection can be browsed and queued to the player from a self served website. Only supports MP3+G (mp3 & corresponding cdg) files.

**_Now built off [glium](https://github.com/tomaka/glium)! No more dependency on SFML, the binaries should run out of the box on any system. Confirmed working on Raspberry Pi 3B + with OpenGL 2.1_**

# Setup
### Linux
- Download latest release binary or compile from source -- `cargo build --release`
- Run `karaoke-rs --help` to see all arguments
- Place your song collection at `~/.local/share/karaoke-rs/songs`, or specify location via `--songs path/to/song/directory`
- Default configuration file is created at `~/.config/karaoke-rs/config.yaml`. This can be copied / changed and specified via `--config path/to/config.yaml`
- You may need to force disable vsync to eliminate flickering, set environment variable `vblank_mode=0`

### Windows
- Download latest release binary or compile from source -- `cargo build --release`
- Double click `karaoke-rs.exe` to run with default configuration. Run from command prompt / powershell `karaoke-rs.exe --help` to see all arguments
- Place your song collection at `%APPDATA%\karaoke-rs\songs`, or specify location via `--songs C:\path\to\song\directory`
- Default configuration file is created at `%APPDATA%\karaoke-rs\config.yaml`. This can be copied / changed and specified via `--config C:\path\to\config.yaml`
- Ensure all paths supplied via argument are absolute from the root of the applicable drive. Relative paths appear to cause program to crash

# TODO
- [x] Finish setting up configuration file, allow specifying song directory and data directory (for collection db file)
- [x] Allow passing config file location as argument
- [x] Bundle website template / static files into build binary, unload them to data path on run, update Rocket to load templates / static files from that path
- [ ] Setup proper logging and error handling


# Screenshots

### Command Line
![cli](/screenshots/cli.png?raw=true)

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
