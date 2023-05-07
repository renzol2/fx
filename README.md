# fx

building and documenting realtime audio effects, and maybe other things if i have time.

this repo contains the following:

## the crate, `fx`

`fx` is a Rust crate which contains DSP functions and building blocks for making basic audio effects.

## example plugins

i've implemented a collection of basic plugins implemented using `nih-plug` to implement and integrate `fx` as VST3/CLAP plugins, which include

- digital effects in `bitcrush`
- chorus/flanger effect in `chorus`
- basic feedback delay in `delay`
- several waveshaper distortion algorithms in `distortion`
- biquad filter implementation in `equalizer`
- classic digital reverb effects in `reverb`
- stereo vibrato effect in `vibrato`

this project is being completed in fulfillment of the senior capstone requirement for the Computer Science + Music degree program at the University of Illinois at Urbana-Champaign.

## licensing

`nih-plug` are licensed under the ISC license. Code that I've ported or copied is cited with the license & author. The rest of the code is licensed under the GPLv3 license.
