# Distortion

A collection of nonlinear waveshaping algorithms for distortion effects.

- [ ] Soft clipping
- [ ] Hard clipping
- [ ] Rectifier (Chowdhury)
- [ ] Dropout (Chowdhury)
- [ ] Double soft clipper (Chowdhury)
- [ ] Wavefolding (Chowdhury)

Additional parameters:

- [ ] Input/output gain
- [ ] Dry/wet mix
- [ ] Pre- and post- filtering (Signalsmith)

## Building

After installing [Rust](https://rustup.rs/), you can compile Distortion as follows:

```shell
cargo xtask bundle distortion --release
```
