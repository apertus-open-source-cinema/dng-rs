use clap::Parser;

use dng::yaml::dumper::IfdYamlDumper;
use dng::yaml::parser::IfdYamlParser;
use std::fs::File;
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
            match ifd {
                Ok(ifd) => println!("{}", IfdYamlDumper::default().dump_ifd(&ifd)),
                Err(e) => println!("{e}"),
            }
        }
        Action::Dir => {}
    }
}
