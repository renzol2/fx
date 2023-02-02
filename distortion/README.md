# Distortion

A collection of nonlinear waveshaping algorithms for distortion effects.

- [x] Soft clipping ([musicdsp](https://www.musicdsp.org/en/latest/Effects/46-waveshaper.html))
- [x] Hard clipping 
- [x] Rectifier (Custom)
- [x] Dropout ([Chowdhury](https://ccrma.stanford.edu/~jatin/papers/Complex_NLs.pdf))
- [x] Double soft clipper ([Chowdhury](https://ccrma.stanford.edu/~jatin/papers/Complex_NLs.pdf))
- [x] Wavefolding ([Chowdhury](https://ccrma.stanford.edu/~jatin/papers/Complex_NLs.pdf))

Additional parameters:

- [x] Input/output gain
- [ ] Mixing of multiple distortion types into one signal (like AnalogObsession's COMPER and its compressor emulations)
- [ ] Pre- and post- filtering (Signalsmith)

DSP related features:

- [ ] Oversampling

## Building

After installing [Rust](https://rustup.rs/), you can compile Distortion as follows:

```shell
cargo xtask bundle distortion --release
```
