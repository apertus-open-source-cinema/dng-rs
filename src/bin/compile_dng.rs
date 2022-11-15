use clap::Parser;
use dng::dng_writer::DngWriter;

use dng::yaml::parser::IfdYamlParser;
use dng::FileType;
use std::fs::{File, OpenOptions};
use std::io::Read;
use std::path::Path;

/// Assemble a DNG file from some of other dng files, plain raw files and metadata
#[derive(clap::Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    action: Action,
}
#[derive(clap::Subcommand)]
enum Action {
    /// compile a DCP (DNG Camera Profile) file from a given textual YAML representation
    Dcp {
        /// input file to get the metadata from
        yaml_file: String,
    },
    /// compile directory (as output by `dump_dng`) to a DNG file
    Dir,
}

fn main() {
    let args = Args::parse();

    match args.action {
        Action::Dcp { yaml_file: file } => {
            let yaml_path = Path::new(&file);
            let mut file = File::open(yaml_path).expect("Cannot find YAML file!");
            let mut contents = String::new();
            file.read_to_string(&mut contents).unwrap();

            let ifd = IfdYamlParser::parse_from_str(&contents);
            let ifd = match ifd {
                Ok(ifd) => ifd,
                Err(e) => panic!("{e}"),
            };

            let dcp_file_path = yaml_path.parent().unwrap().join(format!(
                "{}.dcp",
                yaml_path.file_stem().unwrap().to_str().unwrap()
            ));
            let dcp_file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(dcp_file_path)
                .unwrap();
            DngWriter::write_dng(dcp_file, true, FileType::Dcp, vec![ifd]).unwrap();
        }
        Action::Dir => {}
    }
}
