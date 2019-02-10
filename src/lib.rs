#![crate_type = "lib"]
#![crate_name = "st7735"]

//! This create provides a ST7735 driver to connect to TFT displays.
//!
//! # Examples
//!
//! [todo]

#[macro_use]
extern crate num_derive;

pub mod color;
pub mod command;
pub mod fonts;

use crate::command::{Command, Instruction};
use crate::fonts::font57::Font57;
use crate::fonts::Font;
use num;
use num::integer::sqrt;
use spidev::{Spidev, SpidevOptions, SPI_MODE_0};
use std::cmp::{max, min};
use std::io::prelude::*;
use std::mem::transmute;
use std::thread::sleep;
use std::time::Duration;
use sysfs_gpio::{Direction, Pin};

/// ST7735 driver to connect to TFT displays. The driver allows to draw simple shapes,
/// and reset the display.
///
/// Currently, there is support for using hardware SPI as well as software SPI to
/// communicate to the display. Note that using hardware SPI is much faster and
/// recommended to be used if supported by the connecting device.
///
/// # Examples
///
/// [todo]
///
pub struct ST7734 {
    /// Reset pin.
    rst: Option<Pin>,

    /// SPI clock pin.
    clk: Option<Pin>,

    /// Data/command pin.
    dc: Option<Pin>,

    /// MOSI pin.
    mosi: Option<Pin>,

    /// Hardware SPI
    spi: Option<Spidev>,
}

/// Display orientation.
#[derive(FromPrimitive, ToPrimitive)]
pub enum Orientation {
    Portrait = 0x00,
    Landscape = 0x60,
    PortraitSwapped = 0xC0,
    LandScapeSwapped = 0xA0,
}

impl ST7734 {
    /// Creates a new driver instance that uses hardware SPI.
    ///
    /// # Example
    ///
    /// ```
    /// let mut display = ST7734::new_with_spi("/dev/spidev0.0", 25);
    /// ```
    ///
    pub fn new_with_spi(spi: &str, dc: u64) -> ST7734 {
        // todo: make SPI options configurable
        let mut spi = Spidev::open(spi).expect("error initializing SPI");
        let options = SpidevOptions::new()
            .bits_per_word(8)
            .max_speed_hz(20000000)
            .mode(SPI_MODE_0)
            .build();
        spi.configure(&options).expect("error configuring SPI");

        let dc_pin = Pin::new(dc);
        dc_pin.set_direction(Direction::Out);

        let mut display = ST7734 {
            rst: None,
            clk: None,
            dc: Some(dc_pin),
            mosi: None,
            spi: Some(spi),
        };

        display.init();
        display
    }

    /// Creates a new driver instance that uses software SPI using the provided pins.
    ///
    /// # Example
    ///
    /// ```
    /// let display = ST7734::new(None, 24, 25, 23);
    /// ```
    ///
    pub fn new(rst: Option<u64>, clk: u64, dc: u64, mosi: u64) -> ST7734 {
        let clk_pin = Pin::new(clk);
        clk_pin.set_direction(Direction::Out);
        clk_pin.set_value(0).expect("error while setting clock 0");

        let dc_pin = Pin::new(dc);
        dc_pin.set_direction(Direction::Out);

        let mosi_pin = Pin::new(mosi);
        mosi_pin.set_direction(Direction::Out);

        let rst_pin = match rst {
            Some(r) => {
                let pin = Pin::new(r);
                pin.set_direction(Direction::Out);
                Some(pin)
            }
            None => None,
        };

        let mut display = ST7734 {
            rst: rst_pin,
            clk: Some(clk_pin),
            dc: Some(dc_pin),
            mosi: Some(mosi_pin),
            spi: None,
        };

        display.init();
        display
    }

    /// Runs commands to initialize the display.
    fn init(&mut self) {
        let init_commands: Vec<Command> = vec![
            Command {
                instruction: Instruction::SWRESET,
                delay: Some(200),
                arguments: vec![],
            },
            Command {
                instruction: Instruction::SLPOUT,
                delay: Some(200),
                arguments: vec![],
            },
            Command {
                instruction: Instruction::DISPON,
                delay: Some(100),
                arguments: vec![],
            },
        ];

        self.execute_commands(init_commands);
    }

    /// Pluses the clock one time.
    fn pulse_clock(&self) {
        self.clk
            .unwrap()
            .set_value(1)
            .expect("error while pulsing clock");
        self.clk
            .unwrap()
            .set_value(0)
            .expect("error while pulsing clock");
    }

    /// Writes one byte to the display which can either be a command or data.
    fn write_byte(&mut self, value: u8, data: bool) {
        let mode = match data {
            false => 0,
            true => 1,
        };

        self.dc
            .unwrap()
            .set_value(mode)
            .expect("error while writing byte");

        if let Some(ref mut spi) = self.spi {
            spi.write(&[value]);
        } else {
            let mask = 0x80;
            for bit in 0..8 {
                self.mosi.unwrap().set_value(value & (mask >> bit));
                self.pulse_clock();
            }
        }
    }

    /// Writes a data word to the display.
    fn write_word(&mut self, value: u16) {
        let bytes: [u8; 2] = unsafe { transmute(value.to_be()) };
        self.write_byte(bytes[0], true);
        self.write_byte(bytes[1], true);
    }

    /// Sends a list of commands to the display.
    fn execute_commands(&mut self, commands: Vec<Command>) {
        for cmd in &commands {
            self.execute_command(cmd);
        }
    }

    /// Sends a single command to the display.
    fn execute_command(&mut self, cmd: &Command) {
        self.write_byte(num::ToPrimitive::to_u8(&cmd.instruction).unwrap(), false);

        match cmd.delay {
            Some(d) => {
                if cmd.arguments.len() > 0 {
                    sleep(Duration::from_millis(d));
                }
            }
            None => {
                for argument in &cmd.arguments {
                    self.write_byte(*argument, true);
                }
            }
        }
    }

    /// Sets the color to be used.
    fn write_color(&mut self, color: u32) {
        let bytes: [u8; 4] = unsafe { transmute(color.to_be()) };

        if let Some(ref mut spi) = self.spi {
            self.dc
                .unwrap()
                .set_value(1)
                .expect("error while writing byte");
            spi.write(&[bytes[1], bytes[2], bytes[3]]);
        } else {
            self.write_byte(bytes[1], true);
            self.write_byte(bytes[2], true);
            self.write_byte(bytes[3], true);
        }
    }

    /// Sets the address window for the display.
    fn set_address_window(&mut self, x0: u16, y0: u16, x1: u16, y1: u16) {
        self.write_byte(num::ToPrimitive::to_u8(&Instruction::CASET).unwrap(), false);
        self.write_word(x0);
        self.write_word(x1);
        self.write_byte(num::ToPrimitive::to_u8(&Instruction::RASET).unwrap(), false);
        self.write_word(y0);
        self.write_word(y1);
    }

    /// Changes the display orientation.
    ///
    /// # Example
    ///
    /// ```
    /// let display = ST7734::new(None, 24, 25, 23);
    /// display.set_orientation(&Orientation::Portrait);
    /// ```
    ///
    pub fn set_orientation(&mut self, orientation: &Orientation) {
        let command = Command {
            instruction: Instruction::MADCTL,
            delay: None,
            arguments: vec![num::ToPrimitive::to_u8(orientation).unwrap()],
        };
        self.execute_command(&command);
    }

    /// Draws a single pixel with the specified `color` at the defined coordinates on the display.
    ///
    /// # Example
    ///
    /// ```
    /// let display = ST7734::new(None, 24, 25, 23);
    /// display.draw_pixel(50, 50, 0xFF0000);
    /// ```
    ///
    pub fn draw_pixel(&mut self, x: u16, y: u16, color: u32) {
        self.set_address_window(x, y, x, y);
        self.write_byte(num::ToPrimitive::to_u8(&Instruction::RAMWR).unwrap(), false);
        self.write_color(color);
    }

    /// Draws a rectangle with the specified `color` on the display.
    ///
    /// # Example
    ///
    /// ```
    /// let display = ST7734::new(None, 24, 25, 23);
    /// display.draw_rect(50, 20, 80, 40, 0xFF0000);
    /// ```
    ///
    pub fn draw_rect(&mut self, x0: u16, y0: u16, x1: u16, y1: u16, color: u32) {
        let width = x1 - x0 + 1;
        let height = y1 - y0 + 1;
        self.set_address_window(x0, y0, x1, y1);
        self.write_byte(num::ToPrimitive::to_u8(&Instruction::RAMWR).unwrap(), false);
        for i in 0..(width * height) {
            self.write_color(color);
        }
    }

    /// Draws a horizontal with the specified `color` between the provided coordinates on the display.
    ///
    /// # Example
    ///
    /// ```
    /// let display = ST7734::new(None, 24, 25, 23);
    /// display.draw_horizontal_line(50, 20, 80, 0xFF0000);
    /// ```
    ///
    pub fn draw_horizontal_line(&mut self, x0: u16, x1: u16, y: u16, color: u32) {
        let length = x1 - x0 + 1;
        self.set_address_window(x0, y, x1, y);
        self.write_byte(num::ToPrimitive::to_u8(&Instruction::RAMWR).unwrap(), false);
        // todo: move to draw pixel
        for i in 0..length {
            self.write_color(color);
        }
    }

    /// Draws a vertical with the specified `color` between the provided coordinates on the display.
    ///
    /// # Example
    ///
    /// ```
    /// let display = ST7734::new(None, 24, 25, 23);
    /// display.draw_vertical_line(50, 20, 80, 0xFF0000);
    /// ```
    ///
    pub fn draw_vertical_line(&mut self, x: u16, y0: u16, y1: u16, color: u32) {
        let length = y1 - y0 + 1;
        self.set_address_window(x, y0, x, y1);
        self.write_byte(num::ToPrimitive::to_u8(&Instruction::RAMWR).unwrap(), false);

        // todo: move to draw pixel
        for i in 0..length {
            self.write_color(color);
        }
    }

    /// Draws a line with the specified `color` between the provided coordinates on the display.
    ///
    /// # Example
    ///
    /// ```
    /// let display = ST7734::new(None, 24, 25, 23);
    /// display.draw_line(50, 20, 80, 80, 0xFF0000);
    /// ```
    ///
    pub fn draw_line(&mut self, x0: u16, y0: u16, x1: u16, y1: u16, color: u32) {
        if x0 == x1 {
            self.draw_vertical_line(x0, y0, y1, color);
        } else if y0 == y1 {
            self.draw_horizontal_line(x0, x1, y1, color);
        } else {
            let m = ((max(y1, y0) - min(y0, y1)) as f32) / ((max(x1, x0) - min(x0, x1)) as f32);

            if m < 1.0 {
                for x in x0..=x1 {
                    let y = ((x - x0) as f32) * m + (y0 as f32);
                    self.draw_pixel(x, y as u16, color);
                }
            } else {
                for y in y0..=y1 {
                    let x = ((y - y0) as f32) / m + (x0 as f32);
                    self.draw_pixel(x as u16, y, color);
                }
            }
        }
    }

    /// Draws a circle whose border has the specified `color` around the provided coordinates on the display.
    ///
    /// # Example
    ///
    /// ```
    /// let display = ST7734::new(None, 24, 25, 23);
    /// display.draw_circle(50, 20, 10, 0xFF0000);
    /// ```
    ///
    pub fn draw_circle(&mut self, x_pos: u16, y_pos: u16, radius: u16, color: u32) {
        let x_end = ((std::f32::consts::FRAC_1_SQRT_2 * (radius as f32)) + 1.0) as u16;

        for x in 0..x_end {
            let y = sqrt(radius * radius - x * x) as u16;
            let u_x = x as u16;
            self.draw_pixel(x_pos + u_x, y_pos + y, color);
            self.draw_pixel(x_pos + u_x, y_pos - y, color);
            self.draw_pixel(x_pos - u_x, y_pos + y, color);
            self.draw_pixel(x_pos - u_x, y_pos - y, color);
            self.draw_pixel(x_pos + y, y_pos + u_x, color);
            self.draw_pixel(x_pos + y, y_pos - u_x, color);
            self.draw_pixel(x_pos - y, y_pos + u_x, color);
            self.draw_pixel(x_pos - y, y_pos - u_x, color);
        }
    }

    /// Draws a circle filled with the specified `color` around the provided coordinates on the display.
    ///
    /// # Example
    ///
    /// ```
    /// let display = ST7734::new(None, 24, 25, 23);
    /// display.draw_filled_circle(50, 20, 10, 0xFF0000);
    /// ```
    ///
    pub fn draw_filled_circle(&mut self, x_pos: u16, y_pos: u16, radius: u16, color: u32) {
        let r2 = radius * radius;
        for x in 0..radius {
            let y = sqrt(r2 - x * x);
            let y0 = y_pos - y;
            let y1 = y_pos + y;
            self.draw_vertical_line(x_pos + x, y0, y1, color);
            self.draw_vertical_line(x_pos - x, y0, y1, color);
        }
    }

    /// Draws a character filled with the specified `color` and the defined font on the display.
    ///
    /// # Example
    ///
    /// ```
    /// let display = ST7734::new(None, 24, 25, 23);
    /// display.draw_character(50, 20, 10, 0xFF0000, &Font57);
    /// ```
    ///
    pub fn draw_character<F: Font>(&mut self, c: char, x: u16, y: u16, color: u32, font: F) {
        let character_data = <F as Font>::get_char(c);

        let mut mask = 0x01;

        for row in 0..7 {
            for col in 0..5 {
                let bit = character_data[col] & (mask << row);

                if bit != 0 {
                    self.draw_pixel(x - (col as u16), y - (row as u16), color);
                }
            }
        }
    }

    /// Fills the entire screen with the specified `color`.
    ///
    /// # Example
    ///
    /// ```
    /// let display = ST7734::new(None, 24, 25, 23);
    /// display.fill_screen(0xFF0000);
    /// ```
    ///
    pub fn fill_screen(&mut self, color: u32) {
        self.draw_rect(0, 0, 127, 159, color);
    }

    /// Fills the entire screen black.
    ///
    /// # Example
    ///
    /// ```
    /// let display = ST7734::new(None, 24, 25, 23);
    /// display.clear_screen();
    /// ```
    ///
    pub fn clear_screen(&mut self) {
        self.draw_rect(0, 0, 127, 159, 0x0);
    }
}

//https://github.com/arduino-libraries/TFT/blob/master/src/utility/Adafruit_ST7735.cpp

//const RCMD1: Vec<Command> = vec![
//    Command { instruction: Instruction::SWRESET, delay: true, arguments: [150]},
//    Command { instruction: Instruction::SLPOUT, delay: true, arguments: [255]},
//    Command { instruction: Instruction::FRMCTR1, delay: false, arguments: [0x01, 0x2C, 0x2D]},
//    Command { instruction: Instruction::FRMCTR2, delay: false, arguments: [0x01, 0x2C, 0x2D]},
//    Command { instruction: Instruction::FRMCTR3, delay: false, arguments: [0x01, 0x2C, 0x2D, 0x01, 0x2C, 0x2D]},
//    Command { instruction: Instruction::INVCTR, delay: false, arguments: [0x07]},
//    Command { instruction: Instruction::PWCTR1, delay: false, arguments: [0xA2, 0x02, 0x84]},
//    Command { instruction: Instruction::PWCTR2, delay: false, arguments: [0xC5]},
//    Command { instruction: Instruction::PWCTR3, delay: false, arguments: [0x0A, 0x00]},
//    Command { instruction: Instruction::PWCTR4, delay: false, arguments: [0x8A, 0x2A]},
//    Command { instruction: Instruction::PWCTR5, delay: false, arguments: [0x8A, 0xEE]},
//    Command { instruction: Instruction::VMCTR1, delay: false, arguments: [0x0E]},
//    Command { instruction: Instruction::INVOFF, delay: false, arguments: []},
//    Command { instruction: Instruction::MADCTL, delay: false, arguments: [0xC8]},
//    Command { instruction: Instruction::COLMOD, delay: false, arguments: [0x05]},
//];
//
//const RCMD2_GREEN: Vec<Command> = vec![
//    Command { instruction: Instruction::CASET, delay: false, arguments: [0x00, 0x02, 0x00, 0x7F+0x02]},
//    Command { instruction: Instruction::RASET, delay: false, arguments: [0x00, 0x01, 0x00, 0x9F+0x01]},
//];
//
//const RCMD2_RED: Vec<Command> = vec![
//    Command { instruction: Instruction::CASET, delay: false, arguments: [0x00, 0x00, 0x00, 0x7F]},
//    Command { instruction: Instruction::RASET, delay: false, arguments: [0x00, 0x00, 0x00, 0x9F]},
//];
//
//const RCMD2_GREEN144: Vec<Command> = vec![
//    Command { instruction: Instruction::CASET, delay: false, arguments: [0x00, 0x00, 0x00, 0x7F]},
//    Command { instruction: Instruction::RASET, delay: false, arguments: [0x00, 0x00, 0x00, 0x7F]},
//];
//
//const RCMD2_GREEN160X80: Vec<Command> = vec![
//    Command { instruction: Instruction::CASET, delay: false, arguments: [0x00, 0x00, 0x00, 0x7F]},
//    Command { instruction: Instruction::RASET, delay: false, arguments: [0x00, 0x00, 0x00, 0x9F]},
//];
//
//const RCMD3: Vec<Command> = vec![
//    Command { instruction: Instruction::GMCTRP1, delay: false, arguments: [0x02, 0x1c, 0x07, 0x12, 0x37, 0x32, 0x29, 0x2d, 0x29, 0x25, 0x2B, 0x39, 0x00, 0x01, 0x03, 0x10]},
//    Command { instruction: Instruction::GMCTRN1, delay: false, arguments: [0x03, 0x1d, 0x07, 0x06, 0x2E, 0x2C, 0x29, 0x2D, 0x2E, 0x2E, 0x37, 0x3F, 0x00, 0x00, 0x02, 0x10]},
//    Command { instruction: Instruction::NORON, delay: true, arguments: [10]},
//    Command { instruction: Instruction::DISPON, delay: true, arguments: [100]},
//];
