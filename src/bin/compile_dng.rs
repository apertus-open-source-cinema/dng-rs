use clap::{arg, Parser};
use dng::yaml::IfdYamlParser;
use dng::DngWriter;
use dng::FileType;
use std::fs::{File, OpenOptions};
use std::io::Read;
use std::path::Path;

/// Assemble a DNG file from some of other dng files, plain raw files and metadata
#[derive(clap::Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// input YAML file to get the metadata from
    #[arg(long)]
    yaml: String,

    // write the DCP magic bytes (DNG Camera profile) instead of the DNG ones
    #[arg(long, action)]
    dcp: bool,

    // write a big endian DNG (default: little endian)
    #[arg(short = 'b', long, action)]
    big_endian: bool,
}

fn main() {
    let args = Args::parse();
    let yaml_path = Path::new(&args.yaml);
    let file_type = match args.dcp {
        true => FileType::Dcp,
        false => FileType::Dng,
    };

    let mut file = File::open(yaml_path).expect("Cannot find YAML file!");
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();

    let ifd =
        IfdYamlParser::new(yaml_path.parent().unwrap().to_path_buf()).parse_from_str(&contents);
    let ifd = match ifd {
        Ok(ifd) => ifd,
        Err(e) => panic!("{e}"),
    };

    let dcp_file_path = yaml_path.parent().unwrap().join(format!(
        "{}.{}",
        yaml_path.file_stem().unwrap().to_str().unwrap(),
        file_type.extension(),
    ));
    let dcp_file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(dcp_file_path)
        .unwrap();
    DngWriter::write_dng(dcp_file, !args.big_endian, file_type, vec![ifd]).unwrap();
}
