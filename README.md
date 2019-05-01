# karaoke-rs
A simple, network enabled karaoke player in Rust. 

Your karaoke collection can be browsed and queued to the player from a self served website. Currently only supports MP3+G (mp3 & corresponding cdg) files. Only tested on linux, but pull requests are welcome to get working on OSX & Windows.


## Setup
- Install SFML and CSFML bindings to run, required by rust-sfml -- see [link to help setup](https://github.com/jeremyletang/rust-sfml/wiki/Linux)
- Download latest release binary or compile from source -- `cargo build --release`
- Run `karaoke-rs --help` to see all arguments
- Place your song collection at `~/.local/share/karaoke-rs/songs`, or specify location via `--songs path/to/song/directory`
- Default configuration file is created at `~/.config/karaoke-rs/config.yaml`. This can be copied / changed and specified via `--config path/to/config.yaml`


## TODO
- [x] Finish setting up configuration file, allow specifying song directory and data directory (for collection db file)
- [x] Allow passing config file location as argument
- [x] Bundle website template / static files into build binary, unload them to data path on run, update Rocket to load templates / static files from that path
- [ ] Change collection refresh from on startup to triggered from website
- [ ] Add some stats to the collection database, such as number of times played, last date listened to, date added, etc.
- [ ] Setup proper logging and error handling


## Screenshots

### Command Line
![cli](/screenshots/cli.png?raw=true)

### Songs Page
![songs](/screenshots/songs.png?raw=true)

### Artists Page
![artists](/screenshots/artists.png?raw=true)

### Queue Page
![queue](/screenshots/queue.png?raw=true)

### Player - background color rainbow cycles
![player1](/screenshots/player_1.png?raw=true)

![player2](/screenshots/player_2.png?raw=true)
