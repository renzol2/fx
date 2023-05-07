# Distortion

A collection of nonlinear waveshaping algorithms for distortion effects.

- [x] Soft clipping ([musicdsp](https://www.musicdsp.org/en/latest/Effects/46-waveshaper.html))
- [x] Hard clipping 
- [x] "Fuzzy" rectifier
- [x] Shockley diode rectifier ([Chowdhury](https://ccrma.stanford.edu/~jatin/papers/Complex_NLs.pdf))
- [x] Dropout ([Chowdhury](https://ccrma.stanford.edu/~jatin/papers/Complex_NLs.pdf))
- [x] Double soft clipper ([Chowdhury](https://ccrma.stanford.edu/~jatin/papers/Complex_NLs.pdf))
- [x] Wavefolding ([Chowdhury](https://ccrma.stanford.edu/~jatin/papers/Complex_NLs.pdf))

Additional parameters:

- [x] Input/output gain
- [x] Pre- and post- filtering (Signalsmith)

DSP related features:

- [x] Oversampling

Oversampling is implemented using a DC filter and halfband filter.

Useful links during implementation:

- <https://github.com/Fredemus/va-filter/blob/main/src/resampling.rs>
- <https://en.wikipedia.org/wiki/Half-band_filter>
- <https://www.musicdsp.org/en/latest/Filters/39-polyphase-filters.html>

## Building

After installing [Rust](https://rustup.rs/), you can compile Distortion as follows:

```shell
cargo xtask bundle distortion --release
```
