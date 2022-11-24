# DNG-rs

A pure rust library for reading / writing DNG files providing access to the raw data in a zero-copy friendly way.
It also contains code for reading / writing a human-readable YAML representation of DNG tags / the IFD structure.
The library also supports interacting with DCP (Dng Camera Profile) files, but that is on a best-effort basis since I
was unable to find official documentation on that.

## Tools
This library also contains a pair of cli tools for converting a DNG into a human-readable YAML representation and back.
These are kind of similar to [dcpTool](https://dcptool.sourceforge.net/Usage.html)'s `-d` and `-c` but use YAML rather than XML.

```shell
$ target/debug/dump_dng -h                                           
Dump the IFD metadata of a TIFF / DNG image to a human readable yaml representation

Usage: dump_dng [OPTIONS] <FILE>

Arguments:
  <FILE>  input file to get the metadata from

Options:
  -f, --dump-rational-as-float  convert Rational and SRational types to float for better readability (this is lossy)
  -e, --extract                 extract strips, tiles and larger blobs into a directory. also write the ifd chain as a yaml file there
  -h, --help                    Print help information
  -V, --version                 Print version information

```

```shell
$ target/debug/compile_dng -h 
Assemble a DNG file from some of other dng files, plain raw files and metadata

Usage: compile_dng [OPTIONS] --yaml <YAML>

Options:
      --yaml <YAML>  input YAML file to get the metadata from
      --dcp          
  -b, --big-endian   
  -h, --help         Print help information
  -V, --version      Print version information
```

example:
```shell
$ target/debug/dump_dng src/yaml/testdata/axiom_beta_simulated.dcp -f 
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
