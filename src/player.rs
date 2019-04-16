use std::io;
use std::io::BufReader;
use std::fs::File;
use std::f32::consts;

use sfml::audio::{Sound, SoundBuffer, SoundStatus};
use sfml::graphics::{RenderWindow, Texture, RectangleShape, RenderTarget, Transformable, BlendMode, RenderStates, Transform};
use sfml::system::Vector2f;
use sfml::window::{VideoMode, Style, ContextSettings, Event, Key};
use sfml::system::{sleep, Time};

use image::{GenericImage};

use crate::collection::Kfile;

pub fn play(kfile: Kfile) -> io::Result<()> {
    let desktop = VideoMode::desktop_mode();
    let mut window = RenderWindow::new(
        desktop,
        "Karaoke",
        Style::FULLSCREEN,
        &ContextSettings::default(),
    );

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

    let frame_x_center = desktop.width as f32 * 0.5 - ( frame_x as f32 * scale ) * 0.5;
    let frame_y_center = desktop.height as f32 * 0.5 - ( frame_y as f32 * scale ) * 0.5;
    let frame_center = Vector2f::new(frame_x_center, frame_y_center);
    let frame_size = Vector2f::new(frame_x as f32, frame_y as f32);
    let frame_scale = Vector2f::new(scale, scale);

    let background_x = 1;
    let background_y = 1;

    let mut texture_background = Texture::new(background_x, background_y).unwrap();
    texture_background.set_repeated(true);
    let background_size = Vector2f::new(desktop.width as f32, desktop.height as f32);
    
    let mut i: f32 = 0.0;
    let size: f32 = 4096.0;

    let mut last_sector_no: isize = 0;
    let mut sectors_since: isize = 0;
    let mut interp = cdg_renderer::CdgInterpreter::new();
    let mut res_image = image::RgbaImage::new(300,216);

    song.play();
    'running: loop {    
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
                } => break 'running,
                _ => {}
            }
        }          
        if song.status() == SoundStatus::Stopped {
            break 'running
        }
        sleep(Time::milliseconds(40));
    }
    Ok(())
}