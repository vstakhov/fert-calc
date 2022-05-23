# fert-calc

`fert-calc` is a small and specific utility that I have written to simplify DIY mixing of the aquarium fertilisers based on different components, dosing schemas and so on.
The main goal of this utility is to provide a command line interface to simplify manual calculations for dilutions and dosing.

## Status

This project is created for personal hobby and is not intended to be useful for anyone. It is also in alpha stage, so many functions are yet missing.

## How to use

First, you'd need to [install Rust](https://www.rust-lang.org/tools/install).

The rest is pretty straightforward, just run `cargo install --git https://github.com/vstakhov/fert-calc` to build
this repo from the sources. Then you can use `fert-calc` binary to do your own calculations.

## What it can do

* Tell you what happens if you add some compound to your tank as a dry salt or a solution:
  [![asciicast](https://asciinema.org/a/zHyEh2IILKtfJdL2z8KY7Y7hS.svg)](https://asciinema.org/a/zHyEh2IILKtfJdL2z8KY7Y7hS)
* Tell you what happens if you add specific fertilizer to your tank:
  [![asciicast](https://asciinema.org/a/g34k58KnlEn0x2gKYic7MWCuS.svg)](https://asciinema.org/a/g34k58KnlEn0x2gKYic7MWCuS)
* Tell you how to reach a desired concentration in your tank using a compound or a ready mix (allowing aliases to specify targets):
  [![asciicast](https://asciinema.org/a/SKAiY9cUtMirFh0AJPTmSlB5T.svg)](https://asciinema.org/a/SKAiY9cUtMirFh0AJPTmSlB5T)

You can also take a look at the [embedded database of the fertilizers](https://github.com/vstakhov/fert-calc/blob/master/fertilizers.toml) to get a glue about how to define your own ones. To define your own database you shoul use `--fertilizers-db=your_ferts.toml` option.
Please check the output of the `fert-calc --help` for the list of all available options.

## Roadmap

Large/important features:

* [x] Agricultural fertilizers and percent based fertilizers
* [x] Dose to reach the target mode
* [ ] Dosing regimes: EI/PPS pro/EI low light
* [x] Custom fertilisers and compounds trivial names (urea, CSM+B etc)

Small features:

* [ ] Hydrates support
* [ ] Saving for the input values to avoid extra hassle
* [x] Solution based dilution
* [ ] Solubility warnings
* [ ] Target solution PH