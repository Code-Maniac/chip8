use sdl2::rect::Rect;
use sdl2::render::WindowCanvas;
use sdl2::Sdl;

use super::colors::BLACK;
use super::colors::WHITE;

// 256 bytes for the display
const DISPLAY_WIDTH: usize = 64;
const DISPLAY_HEIGHT: usize = 32;

const DISPLAY_SIZE: usize = DISPLAY_WIDTH * DISPLAY_HEIGHT;

pub struct VideoDevice {
    canvas: WindowCanvas,
    pixelmap: [u8; DISPLAY_SIZE],
    pixelsize: usize,
    dirty: bool,
}

impl VideoDevice {
    pub fn new(sdl_context: &Sdl, pixelsize: usize) -> VideoDevice {
        let video_subsystem = sdl_context.video().unwrap();
        let window = video_subsystem
            .window(
                "CHIP8",
                (DISPLAY_WIDTH * pixelsize) as u32,
                (DISPLAY_HEIGHT * pixelsize) as u32,
            )
            .position_centered()
            .build()
            .expect("Could not initialise video sybsystem");
        let canvas = window
            .into_canvas()
            .build()
            .expect("Could not make window canvas");

        VideoDevice {
            canvas,
            pixelmap: [0; DISPLAY_SIZE],
            pixelsize,
            dirty: true,
        }
    }

    pub fn render(&mut self) {
        if self.dirty {
            let mut rect = Rect::new(0, 0, self.pixelsize as u32, self.pixelsize as u32);
            for i in 0..DISPLAY_SIZE {
                let x = i % DISPLAY_WIDTH;
                let y = i / DISPLAY_WIDTH;

                rect.set_x((x * self.pixelsize) as i32);
                rect.set_y((y * self.pixelsize) as i32);

                let pixel = self.get_pixel(x as u8, y as u8);

                if pixel == 0x0 {
                    self.canvas.set_draw_color(BLACK);
                } else {
                    self.canvas.set_draw_color(WHITE);
                }
                self.canvas.fill_rect(rect).unwrap();
            }
            self.present();
        }
    }

    pub fn clear(&mut self) {
        // set all pixels to 0
        for i in 0..DISPLAY_SIZE {
            self.pixelmap[i] = 0;
        }
        self.dirty = true;
    }

    pub fn get_pixel_byte_addr(&self, x: u8, y: u8) -> usize {
        (x as usize) + ((y as usize) * DISPLAY_WIDTH)
    }

    pub fn get_pixel(&self, x: u8, y: u8) -> u8 {
        let pixel_byte_addr = self.get_pixel_byte_addr(x, y);
        self.pixelmap[pixel_byte_addr]
    }

    pub fn set_pixel(&mut self, x: u8, y: u8, mut val: u8) {
        val &= 0x1;

        let pixel_byte_addr = self.get_pixel_byte_addr(x, y);
        self.pixelmap[pixel_byte_addr] ^= val;

        self.dirty = true;
    }

    pub fn get_width(&self) -> usize {
        DISPLAY_WIDTH
    }

    pub fn get_height(&self) -> usize {
        DISPLAY_HEIGHT
    }

    fn present(&mut self) {
        self.canvas.present();
        self.dirty = false;
    }
}
