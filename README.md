# picolcd114

This crate is a heavily modified fork of the st7789 TFT display driver crate.
It contains support for the Waveshare Pico-LCD 1.14".

The upstream st7789 initialization code did not work on this display, so the
init code was instead replaced by a translation of the demo code provided by
Waveshare from C to Rust. It is unclear what caused the original code to fail,
so this problem will be reinvestigated.

The driver contains a blit_pixels() function that avoids the use of iterators
in order to allow proper DMA, but of course puts the burden of providing a
buffer with the correct pixel data on the caller.

Orientation support has been removed, but could be reinstated if necessary.
The contents of the original README are included below.

# st7789

This is a Rust driver library for ST7789 displays using embedded_graphics, embedded_hal, and no_std, no_alloc support. 
- [Driver documentation](https://docs.rs/st7789). 
- [Examples](https://github.com/almindor/st7789-examples)
- [Display datasheet](https://www.rhydolabz.com/documents/33/ST7789.pdf)

[![ferris-demo](http://objdump.katona.me/ferris_fast.png)](http://objdump.katona.me/ferris_fast.mp4)

## Features

These features are enabled by default:

* `graphics` - embedded-graphics support: pulls in [embedded-graphics](https://crates.io/crates/embedded-graphics) dependency
* `batch` - batch-drawing optimization: pulls in [heapless](https://crates.io/crates/heapless) dependency and allocates 300 bytes for frame buffer in the driver
* `buffer` - use a 128 byte buffer for SPI data transfers

## Status

- [x] Communications via SPI
- [x] Tested with PineTime watch
- [x] Hardware scrolling support
- [ ] Offscreen Buffering

## [Changelog](CHANGELOG.md)

