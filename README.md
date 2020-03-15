# st7789

This is a Rust library for displays using the ST7789 driver with embedded_graphics, embedded_hal, and no_std, no_alloc support. Documentation is available [here](https://docs.rs/st7789). Examples are [here](https://github.com/almindor/st7789-examples)

[![ferris-demo](http://objdump.katona.me/ferris_fast.png)](http://objdump.katona.me/ferris_fast.mp4)

## Features

These features are enabled by default

* `graphics` - embedded-graphics support, pulls in [embedded-graphics](https://crates.io/crates/embedded-graphics) dependency
* `batch` - batch-drawing optimization, pulls in [heapless](https://crates.io/crates/heapless) dependency and allocates 300 bytes for frame buffer in the driver
* `buffer` - use a 128 byte buffer for SPI data transfers

## Changelog

* `v0.2.2` - add buffering for SPI transfers
* `v0.2.1` - use static dispatch for `set_pixels`
* `v0.2.0` - batch support
* `v0.1.0` - initial release

## Roadmap

* `vTBD`   - hardware scolling support
