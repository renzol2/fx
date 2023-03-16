# Equalizer

A generic equalizer implemented using a biquad filter.

Supports 7 filter types:

1. low pass
2. high pass
3. band pass
4. notch
5. peaking (parametric) EQ
6. low shelf
7. high shelf

There are a few known issues with this implementation so far:

- When working with stereo input, low pass and band pass turns stereo input to mono.
- Parametric, low shelf, and high shelf filters don't do anything when "cutting" (when the peak gain is below 0).

## Building

After installing [Rust](https://rustup.rs/), you can compile Equalizer as follows:

```shell
cargo xtask bundle equalizer --release
```
