# Chorus

A chorus effect plugin with the following parameters:

- **gain**: the output gain of the effect
- **rate**: the rate of the LFO, in Hz
- **LFO amount**: the magnitude of pitch variation in the LFO
- **depth**: the amount of chorus to apply
- **width**: the phase offset between the left and right delay line LFOs
- **feedback**: the amount of feedback written into the delay line

## Building

After installing [Rust](https://rustup.rs/), you can compile Chorus as follows:

```shell
cargo xtask bundle chorus --release
```
