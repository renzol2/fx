# Bitcrush

A bitcrusher effect plugin with two parameters:

- **bits**: the simulated audio bit depth
- **floating point constant**: a number to add and subtract from audio. Inspired by tom7's first example of nonlinear functions utilizing the imprecision of half-precision IEEE-754 floating point numbers (which Rust uses for `f32`).

## Building

After installing [Rust](https://rustup.rs/), you can compile Bitcrush as follows:

```shell
cargo xtask bundle bitcrush --release
```
