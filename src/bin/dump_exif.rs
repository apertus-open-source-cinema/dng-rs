use clap::Parser;
use dng::DngFile;

use std::fs::File;

/// Dump the EXIF metadata of a TIFF / DNG image and write it to a yaml file
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// input file to get the metadata from
    file: String,
    #[arg(short = 'f', long, action)]
    dump_rational_as_float: bool,
}

fn main() {
    let args = Args::parse();
    let img_file = File::open(args.file).expect("Cannot find test image!");
    let mut tiff = DngFile::new(img_file).expect("Couldnt parse TIFF file!");

    let mut string = String::new();
    tiff.read_ifd()
        .unwrap()
        .pretty_yaml(&mut string, args.dump_rational_as_float)
        .unwrap();
    print!("{}", string);
}
