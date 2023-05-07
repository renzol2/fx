# Delay

A basic feedback delay effect plugin with the following parameters:

- **feedback**: the gain multiplier for the signal fed back into the delay line
- **dry/wet**: yeah
- **delay time**: the amount of time for the output signal to exit the delay line

## Building

After installing [Rust](https://rustup.rs/), you can compile Delay as follows:

```shell
cargo xtask bundle delay --release
```
