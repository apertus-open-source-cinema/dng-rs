# `dng` &emsp; [![crates-shield]][crates.io] [![docs-shield]][docs.rs]

[crates-shield]: https://img.shields.io/crates/v/dng.svg
[crates.io]: https://crates.io/crates/dng
[docs-shield]: https://img.shields.io/docsrs/dng.svg
[docs.rs]: https://docs.rs/dng

**A pure rust library for reading/writing DNG files providing access to the raw data in a zero-copy friendly way.**
Also containing code for reading/writing a human-readable YAML representation of DNG tags/the IFD structure.

The crate also supports interacting with DCP (DNG Camera Profile) files, but that is on a best-effort basis since I
was unable to find official documentation on that.

## Tools

This library also contains a pair of cli tools for converting a DNG into a human-readable YAML representation and back.

These are kind of similar to [dcpTool](https://dcptool.sourceforge.net/Usage.html)'s `-d` and `-c` but use YAML rather than XML.

### `dump_dng`

```
Dump the IFD metadata of a TIFF/DNG image to a human readable YAML representation

Usage: dump_dng [OPTIONS] <FILE>

Arguments:
  <FILE>  Input file to get the metadata from

Options:
  -f, --dump-rational-as-float  Convert (signed) rational types to float for better readability (this is lossy!)
  -e, --extract                 Extract strips, tiles and larger blobs into a directory; also write the IFD chain as a YAML file there
  -h, --help                    Print help
  -V, --version                 Print version
```

### `compile_dng`

```
Assemble a DNG file from some of other DNG files, plain RAW files and metadata

Usage: compile_dng [OPTIONS] --yaml <YAML>

Options:
      --yaml <YAML>  Input YAML file to get the metadata from
      --dcp          Write the DCP magic bytes (DNG Camera profile) instead of the DNG ones
  -b, --big-endian   Write a big endian DNG (default: little endian)
  -h, --help         Print help
  -V, --version      Print version
```

Example:

```
$ dump_dng src/yaml/testdata/axiom_beta_simulated.dcp -f
UniqueCameraModel: "AXIOM Beta"
ProfileName: "AXIOM Beta spectral simulated"
ProfileEmbedPolicy: allow copying
CalibrationIlluminant1: StandardIlluminantA
ColorMatrix1: [
  2.698, -1.8779, 0.3348,
  0.493, 0.0325, 0.2078,
  0.2645, -0.1286, 0.3895,
]
CalibrationIlluminant2: D65Illuminant
ColorMatrix2: [
  2.5136, -1.2873, -0.1654,
  0.2275, 0.5494, 0.0929,
  0.1393, 0.0697, 0.4617,
]
```

## Current Status

This library should be in a usable state for many applications. However, a more high-level API is not implemented (yet?).

For that (and support for other raw formats) you might want to use [rawloader](https://docs.rs/rawloader/latest/rawloader/).
