use embedded_graphics::drawable::Pixel;
use embedded_graphics::geometry::Dimensions;
use embedded_graphics::image::{Image, ImageDimensions, IntoPixelIter};
use embedded_graphics::pixelcolor::raw::{RawData, RawU16};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::{DrawTarget, Size};
use embedded_graphics::primitives::Rectangle;
use embedded_graphics::style::{PrimitiveStyle, Styled};

use embedded_hal::digital::v2::OutputPin;

use crate::{Error, ST7789};
use display_interface::WriteOnlyDataCommand;

impl<DI, RST, PinE> ST7789<DI, RST>
where
    DI: WriteOnlyDataCommand,
    RST: OutputPin<Error = PinE>,
{
    fn fill_rect(
        &mut self,
        item: &dyn Dimensions,
        colors: &mut dyn Iterator<Item = u16>,
    ) -> Result<(), Error<PinE>> {
        let sx = item.top_left().x as u16;
        let sy = item.top_left().y as u16;
        let ex = item.bottom_right().x as u16;
        let ey = item.bottom_right().y as u16;

        self.set_pixels(sx, sy, ex, ey, colors)
    }
}

impl<DI, RST, PinE> DrawTarget<Rgb565> for ST7789<DI, RST>
where
    DI: WriteOnlyDataCommand,
    RST: OutputPin<Error = PinE>,
{
    type Error = Error<PinE>;

    fn draw_pixel(&mut self, pixel: Pixel<Rgb565>) -> Result<(), Self::Error> {
        let color = RawU16::from(pixel.1).into_inner();
        let x = pixel.0.x as u16;
        let y = pixel.0.y as u16;

        self.set_pixel(x, y, color)
    }

    #[cfg(feature = "batch")]
    fn draw_iter<T>(&mut self, item: T) -> Result<(), Self::Error>
    where
        T: IntoIterator<Item = Pixel<Rgb565>>,
    {
        use crate::batch::DrawBatch;

        self.draw_batch(item)
    }

    fn draw_rectangle(
        &mut self,
        item: &Styled<Rectangle, PrimitiveStyle<Rgb565>>,
    ) -> Result<(), Self::Error> {
        // filled rect can be rendered into frame window directly
        if item.style.fill_color.is_some() {
            let mut colors = item.into_iter().map(|p| RawU16::from(p.1).into_inner());

            self.fill_rect(item, &mut colors)
        } else if let Some(_color) = item.style.stroke_color {
            if item.style.stroke_width == 0 {
                return Ok(()); // nothing to draw
            }
            // let sw = item.style.stroke_width as u16;

            // TODO: construct rectangle as 4 frames
            self.draw_iter(item)
        } else {
            // if we don't know what this rect is, draw individual pixels
            self.draw_iter(item)
        }
    }

    fn draw_image<'a, 'b, I>(&mut self, item: &'a Image<'b, I, Rgb565>) -> Result<(), Self::Error>
    where
        &'b I: IntoPixelIter<Rgb565>,
        I: ImageDimensions,
    {
        // TODO: this is inconsistent in embedded-graphics between Rectangle and Image
        // See: https://github.com/jamwaffles/embedded-graphics/issues/182
        let sx = item.top_left().x as u16;
        let sy = item.top_left().y as u16;
        let ex = (item.bottom_right().x - 1) as u16;
        let ey = (item.bottom_right().y - 1) as u16;
        let colors = item.into_iter().map(|p| RawU16::from(p.1).into_inner());

        self.set_pixels(sx, sy, ex, ey, colors)
    }

    fn size(&self) -> Size {
        Size::new(self.size_x.into(), self.size_y.into()) // visible area, not RAM-pixel size
    }

    fn clear(&mut self, color: Rgb565) -> Result<(), Self::Error>
    where
        Self: Sized,
    {
	let pixelcount = self.size_x * self.size_y;
        let colors = core::iter::repeat(RawU16::from(color).into_inner()).take(pixelcount.into()); // blank entire HW RAM contents
        self.set_pixels(0, 0, self.size_x, self.size_y, colors)
    }
}

pub trait BlitTarget {
	type Error;

	fn blit(&mut self, sx: u16, sy: u16, ex: u16, ey: u16, data: &[u8]) -> Result<(), Self::Error>;
}

impl<DI, RST, PinE> BlitTarget for ST7789<DI, RST>
where
    DI: WriteOnlyDataCommand,
    RST: OutputPin<Error = PinE>,
{
	type Error = Error<PinE>;

	fn blit(&mut self, sx: u16, sy: u16, ex: u16, ey: u16, data: &[u8]) -> Result<(), Self::Error> {
		self.blit_pixels(sx, sy, ex, ey, data)
	}
}
