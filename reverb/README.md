# Reverb

A collection of classic digital reverb algorithms; namely, Schroeder's Freeverb and Moorer's reverb design.

The parameters include:

- **input/output gain**: self-explanatory
- **dry/wet ratio**: also self-explanatory
- **room size**: the amount of feedback in the comb filters, emulating room size
- **dampening**: the damping coefficient for the low-pass element of the comb filters
- **frozen**: option to freeze the reverb (100% feedback, zero damping)
- **reverb type**: option to choose Freeverb or Moorer's reverb
- **width**: amount of separation between left & right reverb outputs

## Building

After installing [Rust](https://rustup.rs/), you can compile Reverb as follows:

```shell
cargo xtask bundle reverb --release
```
