use crossbeam_channel::{select, Receiver, Sender};
use ggez::{
    audio::{SoundData, SoundSource, Source},
    conf,
    event::{
        self,
        winit_event::{Event, KeyboardInput, WindowEvent},
    },
    graphics::{self, DrawParam, Drawable, Rect},
    mint::{Point2, Vector2},
};
use image::GenericImage;
use karaoke::{
    channel::{LiveCommand, PlayerCommand, LIVE_CHANNEL, PLAYER_CHANNEL},
    collection::Kfile,
    queue::PLAY_QUEUE,
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
    pub status: Rc<RefCell<PlayerStatus>>,
    pub player_sender: Sender<PlayerCommand>,
    pub player_receiver: Receiver<PlayerCommand>,
    pub live_sender: Sender<LiveCommand>,
    pub live_receiver: Receiver<LiveCommand>,
    pub queue: Arc<Mutex<Vec<Kfile>>>,
}

impl Player {
    pub fn new() -> Self {
        let status = Rc::from(RefCell::from(PlayerStatus::Stopped));
        let queue = PLAY_QUEUE.clone();
        Player {
            status,
            player_sender: PLAYER_CHANNEL.0.clone(),
            player_receiver: PLAYER_CHANNEL.1.clone(),
            live_sender: LIVE_CHANNEL.0.clone(),
            live_receiver: LIVE_CHANNEL.1.clone(),
            queue,
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
        //Build context and event loop for ggez, get current monitor size
        //and resize window to fullscreen
        let cb = ggez::ContextBuilder::new("karaoke-rs", "tarkah").window_mode(
            conf::WindowMode::default()
                .dimensions(1.0, 1.0)
                .borderless(true),
        );
        let (ctx, events_loop) = &mut cb.build()?;
        let window = graphics::window(ctx);
        let monitor = window.get_current_monitor();
        let win_size = monitor.get_dimensions();
        graphics::set_mode(
            ctx,
            conf::WindowMode::default()
                .dimensions(win_size.width as f32, win_size.height as f32)
                .fullscreen_type(conf::FullscreenType::True)
                .maximized(true),
        )?;
        graphics::set_screen_coordinates(
            ctx,
            Rect::new(0.0, 0.0, win_size.width as f32, win_size.height as f32),
        )?;
        graphics::clear(ctx, graphics::BLACK);

        //Load mp3 into sound buffer, pass into Sound struct which can manage playback
        let mut music_file = File::open(&kfile.mp3_path)?;
        let sound = SoundData::from_read(&mut music_file)?;
        let mut source = Source::from_data(ctx, sound)?;
        source.set_query_interval(std::time::Duration::from_millis(5));

        //Load cdg, create Subchannel Iterator to cycle through cdg sectors
        let cdg = File::open(&kfile.cdg_path)?;
        let mut scsi = cdg::SubchannelStreamIter::new(BufReader::new(cdg));

        //Size of cdg render texture, scaled at 1.5x
        let cdg_x: f32 = 300.0;
        let cdg_y: f32 = 216.0;
        let cdg_scale = 1.5;

        //Calculate center, size and scale of cdg image
        let cdg_x_center = win_size.width as f32 * 0.5 - (cdg_x * cdg_scale) * 0.5;
        let cdg_y_center = win_size.height as f32 * 0.5 - (cdg_y * cdg_scale) * 0.5;
        let cdg_center = Point2 {
            x: cdg_x_center,
            y: cdg_y_center,
        };
        let cdg_scale = Vector2 {
            x: cdg_scale,
            y: cdg_scale,
        };

        //Counter and frequency for rainbow effect
        let mut i: f32 = 0.0;
        let size: f32 = 4096.0;

        //Values to help keep rendered frames in sync with music
        let mut last_sector_no: isize = 0;
        let mut sectors_since: isize = 0;

        //Create CdgInterpreter, which will consume sector commands and produce
        //finished frames which can be copied into RgbaImage. Image data can then
        //be fed into renderable in-GPU-memory image
        let mut cdg_interp = cdg_renderer::CdgInterpreter::new();
        let mut cdg_image = image::RgbaImage::new(300, 216);

        //Play it!
        source.play()?;

        //Loop will get current song position, calculate how many "cdg sectors"
        //have elasped in total (1 sector = 1/75th of a second), and subtract
        //last_sector_no to determine how many sectors worth of cdg commands need
        //to be iterated and processed by the CdgInterpreter. RGBA data can then
        //be copied out of the interpreter and updated to a renderable in-GPU-memory
        //image.
        //
        //Every time a new frame is rendered, the background texture color will
        //update based on a sine wave function to smoothly cycle through the
        //rainbow.
        //
        //Current song can be stopped with either ESC key or receiving a Stop
        //command.
        'player: loop {
            let track_pos = source.elapsed().as_millis() as isize;

            //Offset rendering lyrics by 20 sectors, this syncs lyrics to music
            //almost perfectly
            let calc_sector = (track_pos as f32 / 13.333_333).floor() as isize - 20;

            if calc_sector >= 0 {
                sectors_since = calc_sector - last_sector_no;

                //Iterate each sector, process all commands in CdgInterpreter
                for _ in 0..sectors_since {
                    let sector = scsi.next();

                    if let Some(s) = sector {
                        for cmd in s {
                            cdg_interp.handle_cmd(cmd);
                        }
                    } else {
                        *self.status.borrow_mut() = PlayerStatus::Stopped;
                        break 'player;
                    }
                }

                last_sector_no = calc_sector;
            }

            //Don't start rendering until offset passes 0
            if sectors_since > 0 {
                //Get background color from rainbow cycle, clear to window
                let background_data = rainbow_cycle(&mut i, size);
                let background_color = graphics::Color::from(background_data);
                graphics::clear(ctx, background_color);

                //Get updated cdg frame from interpreter, copy into RGBA image,
                //update to in-GPU-renderable image, draw to window
                cdg_image.copy_from(&cdg_interp, 0, 0);
                let mut cdg_image_gl = ggez::graphics::Image::from_rgba8(
                    ctx,
                    cdg_x as u16,
                    cdg_y as u16,
                    &cdg_image.clone().into_raw()[..],
                )?;
                cdg_image_gl.set_blend_mode(Some(graphics::BlendMode::Replace));
                let draw_param = DrawParam::default().dest(cdg_center).scale(cdg_scale);
                graphics::draw(ctx, &cdg_image_gl, draw_param)?;

                //Render
                graphics::present(ctx)?;
            }

            //Quit song if ESC key pressed
            let mut _break = false;
            events_loop.poll_events(|event| {
                ctx.process_event(&event);
                if let Event::WindowEvent { event, .. } = event {
                    match event {
                        WindowEvent::CloseRequested => {
                            *self.status.borrow_mut() = PlayerStatus::Stopped;
                            _break = true;
                        }
                        WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    virtual_keycode: Some(keycode),
                                    ..
                                },
                            ..
                        } => {
                            if let event::KeyCode::Escape = keycode {
                                *self.status.borrow_mut() = PlayerStatus::Stopped;
                                _break = true;
                            }
                        }
                        _ => (),
                    }
                }
            });
            if _break {
                break 'player;
            };

            //Check to see if Stop command is received for early exit
            select! {
                recv(self.live_receiver) -> cmd => {
                    if cmd.unwrap() == LiveCommand::Stop {
                        *self.status.borrow_mut() = PlayerStatus::Stopped;
                        break 'player;
                    }
                },
                default => {},
            }

            //If song naturally ends, set PlayerStatus to stopped and return
            if source.stopped() {
                *self.status.borrow_mut() = PlayerStatus::Stopped;
                break 'player;
            }

            //Save some CPU time
            ggez::timer::yield_now();
        }

        Ok(())
    }
}

//Sine wave formula for rainbow cycling background color
fn rainbow_cycle(i: &mut f32, size: f32) -> (u8, u8, u8, u8) {
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

    (red, green, blue, 255)
}
