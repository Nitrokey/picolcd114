#![no_std]
// associated re-typing not supported in rust yet
#![allow(clippy::type_complexity)]

//! This crate provides a ST7789 driver to connect to TFT displays.

pub mod instruction;

use crate::instruction::Instruction::*;
use core::iter::once;

use display_interface::DataFormat::{U16BEIter, U8Iter};
use display_interface::WriteOnlyDataCommand;
use embedded_hal::blocking::delay::DelayUs;
use embedded_hal::digital::v2::OutputPin;

#[cfg(feature = "graphics")]
mod graphics;

#[cfg(feature = "batch")]
mod batch;

///
/// ST7789 driver to connect to TFT displays.
///
pub struct ST7789<DI, RST>
where
    DI: WriteOnlyDataCommand,
    RST: OutputPin,
{
    // Display interface
    di: DI,
    // Reset pin.
    rst: RST,
    // Visible size (x, y)
    size_x: u16,
    size_y: u16,
    // Offset to 'true origin' position of controller
    off_x: u16,
    off_y: u16
}

///
/// An error holding its source (pins or SPI)
///
#[derive(Debug)]
pub enum Error<PinE> {
    DisplayError,
    Pin(PinE),
}

impl<DI, RST, PinE> ST7789<DI, RST>
where
    DI: WriteOnlyDataCommand,
    RST: OutputPin<Error = PinE>,
{
    ///
    /// Creates a new ST7789 driver instance
    ///
    /// # Arguments
    ///
    /// * `di` - a display interface for talking with the display
    /// * `rst` - display hard reset pin
    /// * `size_x` - x axis resolution of the display in pixels
    /// * `size_y` - y axis resolution of the display in pixels
    ///
    pub fn new(di: DI, rst: RST, size_x: u16, size_y: u16, off_x: u16, off_y: u16) -> Self {
        Self {
            di,
            rst,
            size_x, size_y,
            off_x, off_y
        }
    }

    ///
    /// Runs commands to initialize the display
    ///
    /// # Arguments
    ///
    /// * `delay_source` - mutable reference to a delay provider
    ///
    pub fn init(&mut self, delay_source: &mut impl DelayUs<u32>) -> Result<(), Error<PinE>> {
        self.hard_reset(delay_source)?;
	self.write_command(MADCTL)?; self.write_data(&[0x70])?;
	self.write_command(COLMOD)?; self.write_data(&[0x55])?; // 16bpp
	self.write_command(PORCTRL)?; self.write_data(&[0x0c, 0x0c, 0x00, 0x33, 0x33])?; // reset default
	self.write_command(GCTRL)?; self.write_data(&[0x35])?; // reset default
	self.write_command(VCOMS)?; self.write_data(&[0x19])?;
	self.write_command(LCMCTRL)?; self.write_data(&[0x2c])?; // reset default
	self.write_command(VDVVRHEN)?; self.write_data(&[0x01])?; // reset default, but 2nd data byte missing (default: 0xff)
	self.write_command(VRHS)?; self.write_data(&[0x12])?;
	self.write_command(VDVS)?; self.write_data(&[0x20])?; // reset default
	self.write_command(FRCTRL2)?; self.write_data(&[0x0f])?; // reset default
	self.write_command(PWCTRL1)?; self.write_data(&[0xa4, 0xa1])?; // reset default
	self.write_command(PVGAMCTRL)?; self.write_data(&[0xd0, 0x04, 0x0d, 0x11, 0x13, 0x2b, 0x3f, 0x54, 0x4c, 0x18, 0x0d, 0x0b, 0x1f, 0x23])?;
	self.write_command(NVGAMCTRL)?; self.write_data(&[0xd0, 0x04, 0x0c, 0x11, 0x13, 0x2c, 0x3f, 0x44, 0x51, 0x2f, 0x1f, 0x1f, 0x20, 0x23])?;
	self.write_command(INVON)?;
	self.write_command(SLPOUT)?;
	self.write_command(DISPON)?;
        delay_source.delay_us(1_000);
        Ok(())
    }

    ///
    /// Performs a hard reset using the RST pin sequence
    ///
    /// # Arguments
    ///
    /// * `delay_source` - mutable reference to a delay provider
    ///
    pub fn hard_reset(&mut self, delay_source: &mut impl DelayUs<u32>) -> Result<(), Error<PinE>> {
        self.rst.set_high().map_err(Error::Pin)?;
        delay_source.delay_us(10); // ensure the pin change will get registered
        self.rst.set_low().map_err(Error::Pin)?;
        delay_source.delay_us(10); // ensure the pin change will get registered
        self.rst.set_high().map_err(Error::Pin)?;
        delay_source.delay_us(10); // ensure the pin change will get registered

        Ok(())
    }

    ///
    /// Sets a pixel color at the given coords.
    ///
    /// # Arguments
    ///
    /// * `x` - x coordinate
    /// * `y` - y coordinate
    /// * `color` - the Rgb565 color value
    ///
    pub fn set_pixel(&mut self, x: u16, y: u16, color: u16) -> Result<(), Error<PinE>> {
        self.set_address_window(x, y, x, y)?;
        self.write_command(RAMWR)?;
        self.di
            .send_data(U16BEIter(&mut once(color)))
            .map_err(|_| Error::DisplayError)?;

        Ok(())
    }

    ///
    /// Sets pixel colors in given rectangle bounds.
    ///
    /// # Arguments
    ///
    /// * `sx` - x coordinate start
    /// * `sy` - y coordinate start
    /// * `ex` - x coordinate end
    /// * `ey` - y coordinate end
    /// * `colors` - anything that can provide `IntoIterator<Item = u16>` to iterate over pixel data
    ///
    pub fn set_pixels<T>(
        &mut self,
        sx: u16,
        sy: u16,
        ex: u16,
        ey: u16,
        colors: T,
    ) -> Result<(), Error<PinE>>
    where
        T: IntoIterator<Item = u16>,
    {
        self.set_address_window(sx, sy, ex, ey)?;
        self.write_command(RAMWR)?;
        self.di
            .send_data(U16BEIter(&mut colors.into_iter()))
            .map_err(|_| Error::DisplayError)
    }

    ///
    /// Blits raw pixel data to the display. The burden of choosing the correct
    /// pixel format is completely on the caller - on the other hand, this is
    /// probably the only way to get acceptable (or *any*, for that matter)
    /// DMA performance.
    ///
    /// # Arguments
    ///
    /// * `sx` - x coordinate start
    /// * `sy` - y coordinate start
    /// * `dx` - width
    /// * `dy` - height
    /// * `data` - u8 slice containing raw pixel data
    ///
    pub fn blit_pixels(
        &mut self,
        sx: u16,
        sy: u16,
        dx: u16,
        dy: u16,
        data: &[u8]
    ) -> Result<(), Error<PinE>> {
        use display_interface::DataFormat::U8;

	if data.len() != (dx*dy*2) as usize {
		return Err(Error::DisplayError);
	}
        self.set_address_window(sx, sy, sx+dx-1, sy+dy-1)?;
        self.write_command(RAMWR)?;
        self.di.send_data(U8(data)).map_err(|_| Error::DisplayError)
    }

    ///
    /// Sets scroll offset "shifting" the displayed picture
    /// # Arguments
    ///
    /// * `offset` - scroll offset in pixels
    ///
    pub fn set_scroll_offset(&mut self, offset: u16) -> Result<(), Error<PinE>> {
        self.write_command(VSCSAD)?;
        self.write_data(&offset.to_be_bytes())
    }

    ///
    /// Release resources allocated to this driver back.
    /// This returns the display interface and the RST pin deconstructing the driver.
    ///
    pub fn release(self) -> (DI, RST) {
        (self.di, self.rst)
    }

    fn write_command(&mut self, command: instruction::Instruction) -> Result<(), Error<PinE>> {
        self.di
            .send_commands(U8Iter(&mut once(command as u8)))
            .map_err(|_| Error::DisplayError)?;
        Ok(())
    }

    fn write_data(&mut self, data: &[u8]) -> Result<(), Error<PinE>> {
        self.di
            .send_data(U8Iter(&mut data.iter().cloned()))
            .map_err(|_| Error::DisplayError)
    }

    // Sets the address window for the display.
    fn set_address_window(
        &mut self,
        sx: u16,
        sy: u16,
        ex: u16,
        ey: u16,
    ) -> Result<(), Error<PinE>> {
        let sx0 = self.off_x + sx;
        let sy0 = self.off_y + sy;
        let ex0 = self.off_x + ex;
        let ey0 = self.off_y + ey;
        self.write_command(CASET)?;
        self.write_data(&sx0.to_be_bytes())?;
        self.write_data(&ex0.to_be_bytes())?;
        self.write_command(RASET)?;
        self.write_data(&sy0.to_be_bytes())?;
        self.write_data(&ey0.to_be_bytes())
    }
}
