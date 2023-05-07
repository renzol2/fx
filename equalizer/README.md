# Equalizer

A straightforward equalizer effect implemented using a biquad filter.

Supports 7 filter types:

1. low pass
2. high pass
3. band pass
4. notch
5. peaking (parametric) EQ
6. low shelf
7. high shelf

## Building

After installing [Rust](https://rustup.rs/), you can compile Equalizer as follows:

```shell
cargo xtask bundle equalizer --release
```
