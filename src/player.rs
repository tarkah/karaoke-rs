use crossbeam_channel::{select, Receiver, Sender};
use glium::{glutin, Surface};
use glutin::{Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use image::GenericImage;
use karaoke::{
    channel::{LiveCommand, PlayerCommand, LIVE_CHANNEL, PLAYER_CHANNEL},
    collection::Kfile,
    queue::PLAY_QUEUE,
};
use rodio::{Sink, Source};
use std::{
    cell::RefCell,
    f32::consts,
    fs::File,
    io::{BufReader, Cursor},
    rc::Rc,
    sync::{
        atomic::{AtomicUsize, Ordering::SeqCst},
        Arc, Mutex,
    },
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

pub struct Player {
    pub status: Rc<RefCell<PlayerStatus>>,
    pub player_sender: Sender<PlayerCommand>,
    pub player_receiver: Receiver<PlayerCommand>,
    pub live_sender: Sender<LiveCommand>,
    pub live_receiver: Receiver<LiveCommand>,
    pub queue: Arc<Mutex<Vec<Kfile>>>,
    pub events_loop: Rc<RefCell<glutin::EventsLoop>>,
    pub display: glium::Display,
    pub dimensions: glutin::dpi::LogicalSize,
    pub background: glium::texture::Texture2d,
}

impl Player {
    pub fn new() -> Self {
        let status = Rc::from(RefCell::from(PlayerStatus::Stopped));
        let queue = PLAY_QUEUE.clone();

        //Setup event loop & display
        let events_loop = glutin::EventsLoop::new();
        let wb =
            glutin::WindowBuilder::new().with_fullscreen(Some(events_loop.get_primary_monitor()));
        let cb = glutin::ContextBuilder::new();
        let display = glium::Display::new(wb, cb, &events_loop).unwrap();

        //Get dimensions of fullscreen window
        let gl_window = display.gl_window();
        let window = gl_window.window();
        let dimensions = window.get_inner_size().unwrap();
        drop(gl_window);

        //Load background image into Texture2d
        let image = image::load(
            Cursor::new(&include_bytes!("../assets/background.png")[..]),
            image::PNG,
        )
        .unwrap()
        .to_rgba();
        let image_dimensions = image.dimensions();
        let image =
            glium::texture::RawImage2d::from_raw_rgba_reversed(&image.into_raw(), image_dimensions);
        let background = glium::texture::Texture2d::new(&display, image).unwrap();

        Player {
            status,
            player_sender: PLAYER_CHANNEL.0.clone(),
            player_receiver: PLAYER_CHANNEL.1.clone(),
            live_sender: LIVE_CHANNEL.0.clone(),
            live_receiver: LIVE_CHANNEL.1.clone(),
            queue,
            events_loop: Rc::from(RefCell::from(events_loop)),
            display,
            dimensions,
            background,
        }
    }

    pub fn run(&self) {
        self.clear_background().unwrap();

        loop {
            select! {
                recv(self.player_receiver) -> cmd => self.process_cmd(cmd.unwrap()),
                default() => self.check_queue(),
            };
            std::thread::sleep(Duration::from_millis(50));

            self.events_loop.borrow_mut().poll_events(|event| {
                if let Event::WindowEvent { event, .. } = event {
                    if let WindowEvent::Focused(_) = event {
                        self.clear_background().unwrap();
                    }
                };
            });
        }
    }

    pub fn clear_background(&self) -> Result<(), failure::Error> {
        let mut frame = self.display.draw();
        frame.clear_color(0.0, 0.0, 0.0, 1.0);

        let background_rect = glium::BlitTarget {
            left: 0,
            bottom: 0,
            width: self.dimensions.width as i32,
            height: self.dimensions.height as i32,
        };
        self.background.as_surface().blit_whole_color_to(
            &frame,
            &background_rect,
            glium::uniforms::MagnifySamplerFilter::Linear,
        );

        frame.finish()?;
        Ok(())
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

    fn empty_stale_live(&self) {
        select! {
            recv(self.live_receiver) -> _ => { },
            default() => { },
        };
    }

    fn play_song(&self, kfile: Kfile) -> Result<(), failure::Error> {
        *self.status.borrow_mut() = PlayerStatus::Playing;

        //Create new output device, load mp3 into sound buffer, decode with rodio, setup periodic access
        //to callback everytime 1ms has passed to track song position for synchronization
        let device = rodio::default_output_device().unwrap();
        let sink = Sink::new(&device);
        let file = File::open(&kfile.mp3_path)?;
        let counter = Arc::from(AtomicUsize::new(0));
        let periodic_counter = counter.clone();
        let access_time = Duration::from_millis(1);
        let source =
            rodio::Decoder::new(BufReader::new(file))?.periodic_access(access_time, move |_| {
                let _ = periodic_counter.fetch_add(1, SeqCst);
            });

        //Load cdg, create Subchannel Iterator to cycle through cdg sectors
        let cdg = File::open(&kfile.cdg_path)?;
        let mut scsi = cdg::SubchannelStreamIter::new(BufReader::new(cdg));

        //Size of cdg render texture, scaled at 1.5x
        let cdg_x: f32 = 300.0;
        let cdg_y: f32 = 216.0;
        let cdg_scale = 1.5;

        //Calculate center cdg image
        let cdg_x_center = self.dimensions.width as f32 * 0.5 - (cdg_x * cdg_scale) * 0.5;
        let cdg_y_center = self.dimensions.height as f32 * 0.5 - (cdg_y * cdg_scale) * 0.5;

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
        sink.append(source);

        //Loop will get current song position, calculate how many "cdg sectors"
        //have elasped in total (1 sector = 1/75th of a second), and subtract
        //last_sector_no to determine how many sectors worth of cdg commands need
        //to be iterated and processed by the CdgInterpreter. RGBA data can then
        //be copied out of the interpreter and blitted to frame surface
        //
        //Every time a new frame is rendered, the background texture color will
        //update based on a sine wave function to smoothly cycle through the
        //rainbow.
        //
        //Current song can be stopped with either ESC key or receiving a Stop
        //command.
        'player: loop {
            let track_pos = counter.load(SeqCst);

            //Offset rendering lyrics by 20 sectors, this syncs lyrics to music
            //almost perfectly
            let calc_sector = (track_pos as f32 / 13.333_333).floor() as isize - 20;

            if calc_sector >= 0 {
                sectors_since = calc_sector - last_sector_no;

                //Iterate each sector, process all commands in CdgInterpreter
                for _ in 0..sectors_since {
                    let sector = scsi.next();

                    //Break the loop once no more sectors exist to render
                    if let Some(s) = sector {
                        for cmd in s {
                            cdg_interp.handle_cmd(cmd);
                        }
                    } else {
                        break 'player;
                    }
                }

                last_sector_no = calc_sector;
            }

            //Don't start rendering until offset passes 0
            if sectors_since > 0 {
                let mut frame = self.display.draw();

                //Get background color from rainbow cycle, clear to window
                let background_data = rainbow_cycle(&mut i, size);
                frame.clear_color(
                    background_data.0,
                    background_data.1,
                    background_data.2,
                    background_data.3,
                );

                //Get updated cdg frame from interpreter, copy into RGBA image,
                //update to texture, blit texture to frame surface with rectangle dimensions
                cdg_image.copy_from(&cdg_interp, 0, 0);
                let cdg_image = glium::texture::RawImage2d::from_raw_rgba_reversed(
                    &cdg_image.clone().into_raw()[..],
                    (cdg_x as u32, cdg_y as u32),
                );
                let cdg_image = glium::Texture2d::new(&self.display, cdg_image)?;
                let cdg_rect = glium::BlitTarget {
                    left: cdg_x_center as u32,
                    bottom: cdg_y_center as u32,
                    width: (cdg_x * cdg_scale) as i32,
                    height: (cdg_y * cdg_scale) as i32,
                };
                cdg_image.as_surface().blit_whole_color_to(
                    &frame,
                    &cdg_rect,
                    glium::uniforms::MagnifySamplerFilter::Linear,
                );

                //Render
                frame.finish()?;
            }

            //Quit song if ESC key pressed
            let mut _break = false;
            self.events_loop.borrow_mut().poll_events(|event| {
                if let Event::WindowEvent { event, .. } = event {
                    match event {
                        WindowEvent::CloseRequested => {
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
                            if let VirtualKeyCode::Escape = keycode {
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
                        break 'player;
                    }
                },
                default => {},
            }

            //Save some CPU time
            std::thread::sleep(Duration::from_millis(10));
        }
        *self.status.borrow_mut() = PlayerStatus::Stopped;
        self.clear_background().unwrap();
        Ok(())
    }
}

//Sine wave formula for rainbow cycling background color
fn rainbow_cycle(i: &mut f32, size: f32) -> (f32, f32, f32, f32) {
    *i = if (*i + 1.0) % size == 0.0 {
        0.0
    } else {
        *i + 1.0
    };
    let red =
        ((consts::PI / size * 2.0 * *i + 0.0 * consts::PI / 3.0).sin() * 127.0).floor() + 128.0;
    let green =
        ((consts::PI / size * 2.0 * *i + 4.0 * consts::PI / 3.0).sin() * 127.0).floor() + 128.0;
    let blue =
        ((consts::PI / size * 2.0 * *i + 8.0 * consts::PI / 3.0).sin() * 127.0).floor() + 128.0;

    (red / 255.0, green / 255.0, blue / 255.0, 1.0)
}
