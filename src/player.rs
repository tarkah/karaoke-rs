use std::io;
use std::io::BufReader;
use std::fs::File;
use std::f32::consts;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;
use std::thread;

use sfml::audio::{Sound, SoundBuffer, SoundStatus};
use sfml::graphics::{RenderWindow, Texture, RectangleShape, RenderTarget, Transformable, BlendMode, RenderStates, Transform, Color};
use sfml::system::Vector2f;
use sfml::window::{VideoMode, Style, ContextSettings, Event, Key};
use sfml::system::{sleep, Time};

use image::{GenericImage};

use crossbeam_channel::{Sender, Receiver};

use karaoke::collection::Kfile;
use karaoke::channel::{LiveCommand, PlayerCommand};
use karaoke::{PLAYER_CHANNEL, LIVE_CHANNEL};

pub fn run() {
    thread::spawn(move || {
        println!("Initializing Player...");
        let player = Player::new();
        println!("Running Player...");
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
    pub background_color: Color,
    //pub queue: Vec<[Kfile]>,
}

impl Player {
    pub fn new() -> Self {
        let win = RenderWindow::new(
            VideoMode::desktop_mode(),
            "Karaoke",
            Style::FULLSCREEN,
            &ContextSettings::default(),
        );
        let window = Rc::from(RefCell::from(win));
        //let window = Arc::from(Mutex::from(win));
        let status = Rc::from(RefCell::from(PlayerStatus::Stopped));
        Player { window: window, status: status,
                player_sender: PLAYER_CHANNEL.0.clone(), player_receiver: PLAYER_CHANNEL.1.clone(),
                live_sender: LIVE_CHANNEL.0.clone(), live_receiver: LIVE_CHANNEL.1.clone(),
                background_color: Color::BLACK}
    }

    pub fn run(&self) {
        loop {
            select! {
                recv(self.player_receiver) -> cmd => self.process_cmd(cmd.unwrap()),
            };
            std::thread::sleep(Duration::from_millis(1000));
        }        
    }

    pub fn stop(&self) {
        self.live_sender.send(LiveCommand::Stop).unwrap();        
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

    fn empty_stale_live(&self) {
        select! {
            recv(self.live_receiver) -> _ => println!("Clearing stale command in live channel."),
            default() => println!("No stale command in live channel."),
        };
    }

    fn play_song(&self, kfile: Kfile) -> io::Result<()> {
        let mut window = self.window.borrow_mut();
        let size = window.size();

        let mut music_data = Vec::new();
        let mut music_sample: i32 = 0;
        let mut music_channels: usize = 0;

        {
            let music_file = File::open(kfile.mp3_path).unwrap();
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

        let sb = SoundBuffer::from_samples(&music_data[..], music_channels as u32, music_sample as u32).unwrap();
        let mut song = Sound::with_buffer(&sb);  

        let infile = File::open(kfile.cdg_path)?;
        let mut scsi = cdg::SubchannelStreamIter::new(BufReader::new(infile));    

        let frame_x = 300;
        let frame_y = 216;
        let scale = 1.5;
        let mut texture_frame = Texture::new(frame_x, frame_y).unwrap();
        texture_frame.set_smooth(true);

        let frame_x_center = size.x as f32 * 0.5 - ( frame_x as f32 * scale ) * 0.5;
        let frame_y_center = size.y as f32 * 0.5 - ( frame_y as f32 * scale ) * 0.5;
        let frame_center = Vector2f::new(frame_x_center, frame_y_center);
        let frame_size = Vector2f::new(frame_x as f32, frame_y as f32);
        let frame_scale = Vector2f::new(scale, scale);

        let background_x = 1;
        let background_y = 1;

        let mut texture_background = Texture::new(background_x, background_y).unwrap();
        texture_background.set_repeated(true);
        let background_size = Vector2f::new(size.x as f32, size.y as f32);
        
        let mut i: f32 = 0.0;
        let size: f32 = 4096.0;

        let mut last_sector_no: isize = 0;
        let mut sectors_since: isize = 0;
        let mut interp = cdg_renderer::CdgInterpreter::new();
        let mut res_image = image::RgbaImage::new(300,216);

        song.play();
        loop {    
            let track_pos = song.playing_offset().as_milliseconds() as isize;
            let calc_sector = (track_pos as f32 / 13.33333333).floor() as isize - 15;
            
            if calc_sector >= 0 {
                sectors_since = calc_sector - last_sector_no;         
                last_sector_no = calc_sector;

                for _ in 0..sectors_since { 
                    let sector = scsi.next().unwrap();

                    for cmd in sector {
                        interp.handle_cmd(cmd);
                    }    
                }
            }       

            if sectors_since > 0 {
                i = if (i + 1.0) % size == 0.0 { 0.0 } else { i + 1.0 };
                let red   = (((consts::PI / size * 2.0 * i + 0.0*consts::PI/3.0).sin() * 127.0).floor() + 128.0) as u8;
                let green = (((consts::PI / size * 2.0 * i + 4.0*consts::PI/3.0).sin() * 127.0).floor() + 128.0) as u8;
                let blue  = (((consts::PI / size * 2.0 * i + 8.0*consts::PI/3.0).sin() * 127.0).floor() + 128.0) as u8;
                let background_data = [ red, green, blue, 255 ];

                texture_background.update_from_pixels(&background_data, 1, 1, 0, 0);        
                let mut background_rect = RectangleShape::with_texture(&texture_background);
                
                background_rect.set_size(background_size);
                window.draw(&background_rect);

                res_image.copy_from(&interp, 0, 0);
                let image = res_image.clone();
                
                let data = image.into_raw();
                texture_frame.update_from_pixels(&data[..], 300, 216, 0, 0);            
                let mut frame_rect = RectangleShape::with_texture(&texture_frame);
                frame_rect.set_size(frame_size);
                frame_rect.set_scale(frame_scale);
                frame_rect.set_position(frame_center);
                let render_state = RenderStates::new(BlendMode::NONE, Transform::default(), None, None);
                window.draw_with_renderstates(&frame_rect, render_state);

                window.display();
            }
            while let Some(event) = window.poll_event() {
                match event {
                    Event::Closed |
                    Event::KeyPressed {
                        code: Key::Escape, ..
                    } => { 
                        *self.status.borrow_mut() = PlayerStatus::Stopped;
                        return Ok(())
                        },
                    _ => {}
                }
            }            
            if song.status() == SoundStatus::Stopped {
                *self.status.borrow_mut() = PlayerStatus::Stopped;
                return Ok(())
            }
            select! {
                recv(self.live_receiver) -> cmd => {
                    if cmd.unwrap() == LiveCommand::Stop {
                        *self.status.borrow_mut() = PlayerStatus::Stopped;
                        return Ok(())
                    }
                },
                default => {},
            }
            sleep(Time::milliseconds(40));
        }
    }    
}