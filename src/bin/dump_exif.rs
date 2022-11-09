use clap::Parser;
use dng::tiff::TiffFile;
use serde_yaml;
use std::fs::File;

/// Dump the EXIF metadata of a TIFF / DNG image and write it to a yaml file
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// input file to get the metadata from
    file: String,
}

fn main() {
    let args = Args::parse();
    let img_file = File::open(args.file).expect("Cannot find test image!");
    let mut tiff = TiffFile::new(img_file).expect("Couldnt parse TIFF file!");

    let mut string = String::new();
    &tiff.get_exif_metadata().unwrap().pretty_yaml(&mut string);
    println!("{}", string);
}
