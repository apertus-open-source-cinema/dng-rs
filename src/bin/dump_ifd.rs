use clap::Parser;
use dng::yaml_dumper::YamlDumper;
use dng::DngFile;
use std::fs;
use std::fs::File;
use std::path::Path;

/// Dump the IFD metadata of a TIFF / DNG image to a human readable yaml representation
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// input file to get the metadata from
    file: String,
    /// convert Rational and SRational types to float for better readability (this is lossy)
    #[arg(short = 'f', long, action)]
    dump_rational_as_float: bool,
    /// extract strips, tiles and larger blobs into a directory. also write the ifd chain as a yaml file there
    #[arg(short = 'e', long, action)]
    extract: bool,
}

fn main() {
    let args = Args::parse();
    let img_file_path = Path::new(&args.file);
    let img_file = File::open(img_file_path).expect("Cannot find test image!");
    let mut tiff = DngFile::new(img_file).expect("Couldnt parse TIFF file!");

    let yaml_dumper = YamlDumper {
        dump_rational_as_float: args.dump_rational_as_float,
    };
    let ifd_yaml = yaml_dumper.dump_ifd(&tiff.read_ifd().unwrap());

    if args.extract {
        let basename = img_file_path
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        let dir = img_file_path
            .parent()
            .unwrap()
            .join(format!("{basename}_extracted"));
        fs::create_dir(&dir).unwrap();
        fs::write(dir.join("ifds.yml"), ifd_yaml).expect("Unable to write file");
    } else {
        print!("{}", ifd_yaml);
    }
}
