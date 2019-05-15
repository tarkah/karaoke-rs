use crossbeam_channel::{select, Receiver, Sender};
use image::GenericImage;
use karaoke::{
    channel::{LiveCommand, PlayerCommand, LIVE_CHANNEL, PLAYER_CHANNEL},
    collection::Kfile,
    queue::PLAY_QUEUE,
};
use sfml::{
    audio::{Sound, SoundBuffer, SoundStatus},
    graphics::{
        BlendMode, Color, RectangleShape, RenderStates, RenderTarget, RenderWindow, Texture,
        Transform, Transformable,
    },
    system::{sleep, Time, Vector2f},
    window::{ContextSettings, Event, Key, Style, VideoMode},
};
use std::{
    cell::RefCell,
    f32::consts,
    fs::File,
    io::BufReader,
    rc::Rc,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

pub fn run() {
    thread::spawn(move || {
        let player = Player::new();
        player.run();
    });
}

#[derive(Eq, PartialEq, Debug)]
pub enum PlayerStatus {
    Playing,
    Stopped,
}

#[derive(Debug)]
pub struct Player {
    pub window: Rc<RefCell<RenderWindow>>,
    pub status: Rc<RefCell<PlayerStatus>>,
    pub player_sender: Sender<PlayerCommand>,
    pub player_receiver: Receiver<PlayerCommand>,
    pub live_sender: Sender<LiveCommand>,
    pub live_receiver: Receiver<LiveCommand>,
    pub queue: Arc<Mutex<Vec<Kfile>>>,
    pub background_color: Color,
}

impl Player {
    pub fn new() -> Self {
        let mut win = RenderWindow::new(
            VideoMode::desktop_mode(),
            "Karaoke",
            Style::FULLSCREEN,
            &ContextSettings::default(),
        );

        let background_color = Color::BLACK;
        win.clear(&background_color);
        win.display();

        let window = Rc::from(RefCell::from(win));
        let status = Rc::from(RefCell::from(PlayerStatus::Stopped));
        let queue = PLAY_QUEUE.clone();
        Player {
            window,
            status,
            player_sender: PLAYER_CHANNEL.0.clone(),
            player_receiver: PLAYER_CHANNEL.1.clone(),
            live_sender: LIVE_CHANNEL.0.clone(),
            live_receiver: LIVE_CHANNEL.1.clone(),
            queue,
            background_color,
        }
    }

    pub fn run(&self) {
        loop {
            select! {
                recv(self.player_receiver) -> cmd => self.process_cmd(cmd.unwrap()),
                default() => self.check_queue(),
            };
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    pub fn stop(&self) {
        self.live_sender.send(LiveCommand::Stop).unwrap();
    }

    pub fn check_queue(&self) {
        let mut queue = self.queue.lock().unwrap();
        if queue.is_empty() {
            drop(queue);
            return;
        }
        let kfile = queue.remove(0);
        drop(queue);
        self.play(kfile);
    }

    pub fn play(&self, kfile: Kfile) {
        std::thread::sleep(Duration::from_millis(100));
        if *self.status.borrow() == PlayerStatus::Playing {
            self.stop();
        }
        std::thread::sleep(Duration::from_millis(100));
        self.empty_stale_live();
        self.play_song(kfile).unwrap();
        self.window.borrow_mut().clear(&self.background_color);
        self.window.borrow_mut().display();
    }

    fn process_cmd(&self, cmd: PlayerCommand) {
        match cmd {
            PlayerCommand::Play { kfile } => self.play(kfile),
        }
    }

    //
    fn empty_stale_live(&self) {
        select! {
            recv(self.live_receiver) -> _ => { },
            default() => { },
        };
    }

    fn play_song(&self, kfile: Kfile) -> Result<(), failure::Error> {
        //Get SFML window to render to
        let mut window = self.window.borrow_mut();
        let win_size = window.size();

        //Load mp3 into sound buffer, pass into Sound struct which can manage playback
        let sound_buffer = get_sound_buffer(&kfile)?;
        let mut song = Sound::with_buffer(&sound_buffer);

        //Load cdg, create Subchannel Iterator to cycle through cdg sectors
        let cdg = File::open(&kfile.cdg_path)?;
        let mut scsi = cdg::SubchannelStreamIter::new(BufReader::new(cdg));

        //Size of cdg render texture, scaled at 1.5x
        let cdg_x = 300;
        let cdg_y = 216;
        let cdg_scale = 1.5;
        let mut cdg_texture = Texture::new(cdg_x, cdg_y).unwrap();
        cdg_texture.set_smooth(true);

        //Calculate center, create vector points for center, size and scale of cdg texture
        let cdg_x_center = win_size.x as f32 * 0.5 - (cdg_x as f32 * cdg_scale) * 0.5;
        let cdg_y_center = win_size.y as f32 * 0.5 - (cdg_y as f32 * cdg_scale) * 0.5;
        let cdg_center = Vector2f::new(cdg_x_center, cdg_y_center);
        let cdg_size = Vector2f::new(cdg_x as f32, cdg_y as f32);
        let cdg_scale = Vector2f::new(cdg_scale, cdg_scale);

        //Background texture is just 1 pixel color, repeated
        let background_x = 1;
        let background_y = 1;
        let mut background_texture = Texture::new(background_x, background_y).unwrap();
        background_texture.set_repeated(true);
        let background_size = Vector2f::new(win_size.x as f32, win_size.y as f32);

        //Counter and frequency for rainbow effect
        let mut i: f32 = 0.0;
        let size: f32 = 4096.0;

        //Values to help keep rendered frames in sync with music
        let mut last_sector_no: isize = 0;
        let mut sectors_since: isize = 0;

        //Create CdgInterpreter, which will consume sector commands and produce
        //finished frames which can be copied into RgbaImage. Image data can then
        //be fed into cdg_texture
        let mut cdg_interp = cdg_renderer::CdgInterpreter::new();
        let mut cdg_image = image::RgbaImage::new(300, 216);

        //Play it!
        song.play();

        //Loop will get current song position, calculate how many "cdg sectors"
        //have elasped in total (1 sector = 1/75th of a second), and subtract
        //last_sector_no to determine how many sectors worth of cdg commands need
        //to be iterated and processed by the CdgInterpreter. RGBA data can then
        //be copied out of the interpreter and updated to the cdg_texture.
        //
        //Every time a new frame is rendered, the background texture color will
        //update based on a sine wave function to smoothly cycle through the
        //rainbow.
        //
        //Current song can be stopped with either ESC key or receiving a Stop
        //command.
        loop {
            let track_pos = song.playing_offset().as_milliseconds() as isize;

            //Offset rendering lyrics by 27 sectors, this syncs lyrics to music
            //almost perfectly
            let calc_sector = (track_pos as f32 / 13.333_333).floor() as isize - 27;

            if calc_sector >= 0 {
                sectors_since = calc_sector - last_sector_no;

                //Iterate each sector, process all commands in CdgInterpreter
                for _ in 0..sectors_since {
                    let sector = scsi.next().unwrap();

                    for cmd in sector {
                        cdg_interp.handle_cmd(cmd);
                    }
                }

                last_sector_no = calc_sector;
            }

            //Don't start rendering until offset passes 0
            if sectors_since > 0 {
                //Get background data from rainbow cycle, update texture, draw
                //to window
                let background_data = rainbow_cycle(&mut i, size);
                background_texture.update_from_pixels(&background_data, 1, 1, 0, 0);
                let mut background_rect = RectangleShape::with_texture(&background_texture);
                background_rect.set_size(background_size);
                window.draw(&background_rect);

                //Get updated cdg frame from interpreter, clone into RGBA image,
                //get data from image, update texture, draw to window
                cdg_image.copy_from(&cdg_interp, 0, 0);
                let _image = cdg_image.clone();
                let data = _image.into_raw();
                cdg_texture.update_from_pixels(&data[..], 300, 216, 0, 0);
                let mut cdg_rect = RectangleShape::with_texture(&cdg_texture);
                cdg_rect.set_size(cdg_size);
                cdg_rect.set_scale(cdg_scale);
                cdg_rect.set_position(cdg_center);
                let render_state =
                    RenderStates::new(BlendMode::NONE, Transform::default(), None, None);
                window.draw_with_renderstates(&cdg_rect, render_state);

                //Render updated background and cdg textures
                window.display();
            }

            //Quit song if ESC key pressed
            while let Some(event) = window.poll_event() {
                match event {
                    Event::Closed
                    | Event::KeyPressed {
                        code: Key::Escape, ..
                    } => {
                        *self.status.borrow_mut() = PlayerStatus::Stopped;
                        return Ok(());
                    }
                    _ => {}
                }
            }

            //Check to see if Stop command is received for early exit
            select! {
                recv(self.live_receiver) -> cmd => {
                    if cmd.unwrap() == LiveCommand::Stop {
                        *self.status.borrow_mut() = PlayerStatus::Stopped;
                        return Ok(())
                    }
                },
                default => {},
            }

            //If song naturally ends, set PlayerStatus to stopped and return
            if song.status() == SoundStatus::Stopped {
                *self.status.borrow_mut() = PlayerStatus::Stopped;
                return Ok(());
            }

            //Save some precious CPU time
            sleep(Time::milliseconds(40));
        }
    }
}

//Open mp3 file, load into memory, decode and return resulting SoundBuffer
fn get_sound_buffer(kfile: &Kfile) -> Result<SoundBuffer, failure::Error> {
    let mut music_data = Vec::new();
    let mut music_sample: i32 = 0;
    let mut music_channels: usize = 0;

    {
        let music_file = File::open(&kfile.mp3_path)?;
        let reader = BufReader::new(music_file);
        let mut decoder = minimp3::Decoder::new(reader);
        let frame_count = 0;
        while let Ok(frame) = decoder.next_frame() {
            music_data.append(&mut frame.data.clone());
            if frame_count == 0 {
                music_sample = frame.sample_rate;
                music_channels = frame.channels;
            }
        }
    }

    Ok(
        SoundBuffer::from_samples(&music_data[..], music_channels as u32, music_sample as u32)
            .unwrap(),
    )
}

//Sine wave formula for rainbow cycling background color
fn rainbow_cycle(i: &mut f32, size: f32) -> [u8; 4] {
    *i = if (*i + 1.0) % size == 0.0 {
        0.0
    } else {
        *i + 1.0
    };
    let red = (((consts::PI / size * 2.0 * *i + 0.0 * consts::PI / 3.0).sin() * 127.0).floor()
        + 128.0) as u8;
    let green = (((consts::PI / size * 2.0 * *i + 4.0 * consts::PI / 3.0).sin() * 127.0).floor()
        + 128.0) as u8;
    let blue = (((consts::PI / size * 2.0 * *i + 8.0 * consts::PI / 3.0).sin() * 127.0).floor()
        + 128.0) as u8;

    [red, green, blue, 255]
}
