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
- [ ] Mixing of multiple distortion types into one signal (like AnalogObsession's COMPER and its compressor emulations)
- [ ] Pre- and post- filtering (Signalsmith)

DSP related features:

- [ ] Oversampling

## Building

After installing [Rust](https://rustup.rs/), you can compile Distortion as follows:

```shell
cargo xtask bundle distortion --release
```
