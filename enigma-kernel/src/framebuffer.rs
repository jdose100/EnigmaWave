use bootloader_api::info::{FrameBufferInfo, PixelFormat};
use noto_sans_mono_bitmap::{
    FontWeight, RasterHeight, RasterizedChar, get_raster, get_raster_width,
};

use lazy_static::lazy_static;
use spin::Mutex;

static mut WRITER_INIT: bool = false;
lazy_static! {
    pub static ref WRITER: Mutex<FrameBufferWriter> =
        unsafe { Mutex::new(FrameBufferWriter::new()) };
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::framebuffer::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    use core::fmt::Write;
    use x86_64::instructions::interrupts::without_interrupts;

    without_interrupts(|| {
        if unsafe { WRITER_INIT } {
            WRITER.lock().write_fmt(args).unwrap();
        }
    });
}

/// Additional vertical space between lines
const LINE_SPACING: usize = 2;
/// Additional horizontal space between characters.
const LETTER_SPACING: usize = 0;

/// Padding from the border. Prevent that font is too close to border.
const BORDER_PADDING: usize = 1;

/// Height of each char raster. The font size is ~0.84% of this. Thus, this is the line height that
/// enables multiple characters to be side-by-side and appear optically in one line in a natural way.
const CHAR_RASTER_HEIGHT: RasterHeight = RasterHeight::Size16;

/// The width of each single symbol of the mono space font.
const CHAR_RASTER_WIDTH: usize = get_raster_width(FontWeight::Regular, CHAR_RASTER_HEIGHT);

/// Backup character if a desired symbol is not available by the font.
/// The '�' character requires the feature "unicode-specials".
const BACKUP_CHAR: char = '�';

/// Returns the raster of the given char or the raster of [`BACKUP_CHAR`].
fn get_char_raster(c: char) -> RasterizedChar {
    fn get(c: char) -> Option<RasterizedChar> {
        get_raster(c, FontWeight::Regular, CHAR_RASTER_HEIGHT)
    }
    get(c).unwrap_or_else(|| get(BACKUP_CHAR).expect("Should get raster of backup char."))
}

pub struct FrameBufferWriter {
    framebuffer: &'static mut [u8],
    info: FrameBufferInfo,
    x_pos: usize,
    y_pos: usize,
}

unsafe impl Send for FrameBufferWriter {}
unsafe impl Sync for FrameBufferWriter {}

impl FrameBufferWriter {
    /// Создает новый регистратор, который использует данный фреймбуфер.
    ///
    /// ## Safety
    ///
    /// После вызова этой функции должен идти вызов функции инициализации,
    /// так-как отсутсвия вызова этой функции несет не определенное поведение.
    pub const unsafe fn new() -> Self {
        static mut ZERO: &mut [u8] = &mut [0];
        let logger: Self;

        #[allow(clippy::deref_addrof)]
        unsafe {
            logger = Self {
                framebuffer: &mut *&raw mut ZERO,
                info: FrameBufferInfo {
                    byte_len: 0,
                    width: 0,
                    height: 0,
                    pixel_format: PixelFormat::U8,
                    bytes_per_pixel: 0,
                    stride: 0,
                },
                x_pos: 0,
                y_pos: 0,
            };
        }

        logger
    }

    fn init(&mut self, framebuffer: &'static mut [u8], info: FrameBufferInfo) {
        self.framebuffer = framebuffer;
        self.info = info;
        self.clear();
    }

    fn newline(&mut self) {
        self.y_pos += CHAR_RASTER_HEIGHT.val() + LINE_SPACING;
        self.carriage_return()
    }

    fn carriage_return(&mut self) {
        self.x_pos = BORDER_PADDING;
    }

    /// Erases all text on the screen. Resets `self.x_pos` and `self.y_pos`.
    pub fn clear(&mut self) {
        self.x_pos = BORDER_PADDING;
        self.y_pos = BORDER_PADDING;
        self.framebuffer.fill(0);
    }

    fn width(&self) -> usize {
        self.info.width
    }

    fn height(&self) -> usize {
        self.info.height
    }

    /// Writes a single char to the framebuffer. Takes care of special control characters, such as
    /// newlines and carriage returns.
    fn write_char(&mut self, c: char) {
        match c {
            '\n' => self.newline(),
            '\r' => self.carriage_return(),
            c => {
                let new_xpos = self.x_pos + CHAR_RASTER_WIDTH;
                if new_xpos >= self.width() {
                    self.newline();
                }

                let new_ypos = self.y_pos + CHAR_RASTER_HEIGHT.val() + BORDER_PADDING;

                if new_ypos >= self.height() {
                    self.clear();
                }

                self.write_rendered_char(get_char_raster(c));
            }
        }
    }

    /// Prints a rendered char into the framebuffer.
    /// Updates `self.x_pos`.
    fn write_rendered_char(&mut self, rendered_char: RasterizedChar) {
        for (y, row) in rendered_char.raster().iter().enumerate() {
            for (x, byte) in row.iter().enumerate() {
                self.write_pixel(self.x_pos + x, self.y_pos + y, *byte);
            }
        }
        self.x_pos += rendered_char.width() + LETTER_SPACING;
    }

    fn write_pixel(&mut self, x: usize, y: usize, intensity: u8) {
        let pixel_offset = y * self.info.stride + x;
        let color = match self.info.pixel_format {
            PixelFormat::Rgb => [intensity, intensity, intensity / 2, 0],
            PixelFormat::Bgr => [intensity / 2, intensity, intensity, 0],
            PixelFormat::U8 => [if intensity > 200 { 0xf } else { 0 }, 0, 0, 0],
            other => {
                // set a supported (but invalid) pixel format before panicking to avoid a double
                // panic; it might not be readable though
                self.info.pixel_format = PixelFormat::Rgb;
                panic!("pixel format {:?} not supported in logger", other)
            }
        };
        let bytes_per_pixel = self.info.bytes_per_pixel;
        let byte_offset = pixel_offset * bytes_per_pixel;
        self.framebuffer[byte_offset..(byte_offset + bytes_per_pixel)]
            .copy_from_slice(&color[..bytes_per_pixel]);
        let _ = unsafe { core::ptr::read_volatile(&self.framebuffer[byte_offset]) };
    }
}

impl core::fmt::Write for FrameBufferWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for c in s.chars() {
            self.write_char(c);
        }
        Ok(())
    }
}

pub fn init(framebuffer: &'static mut [u8], info: FrameBufferInfo) {
    WRITER.lock().init(framebuffer, info);
    unsafe {
        WRITER_INIT = true;
    }
}

// -- TEST ZONE -- //

#[test_case]
fn test_print_simple() {
    println!("test_print_simple output");
}

#[test_case]
fn test_print_many() {
    for _ in 0..255 {
        println!("test_print_many output");
    }
}
