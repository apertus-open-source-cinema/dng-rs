use clap::Parser;
use dng::ifd::IfdValue;
use dng::ifd_tag_data::tag_info_parser::IfdTypeInterpretation;
use dng::yaml_dumper::YamlDumper;
use dng::DngFile;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::Arc;

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
    let dng = Arc::new(DngFile::new(img_file).expect("Couldnt parse TIFF file!"));

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
        fs::create_dir_all(&dir).unwrap();

        let dir_clone = dir.clone();
        let dng_clone = dng.clone();
        let yaml_dumper = YamlDumper {
            dump_rational_as_float: args.dump_rational_as_float,
            visitor: Some(Arc::new(move |entry| {
                if matches!(
                    entry.tag.get_known_type_interpretation(),
                    Some(IfdTypeInterpretation::Blob)
                ) {
                    let path = dir_clone.join(entry.path.string_with_separator("_"));
                    OpenOptions::new()
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .open(path.clone())
                        .unwrap()
                        .write(&entry.value.as_bytes().unwrap())
                        .unwrap();
                    Some(format!(
                        "file://{}",
                        path.strip_prefix(dir_clone.clone())
                            .unwrap()
                            .to_str()
                            .unwrap()
                    ))
                } else if matches!(
                    entry.tag.get_known_type_interpretation(),
                    Some(IfdTypeInterpretation::Offsets { .. })
                ) && !matches!(entry.value, IfdValue::List(_))
                {
                    let path = dir_clone.join(entry.path.string_with_separator("_"));
                    let buffer_size = dng_clone
                        .needed_buffer_size_for_blob(&entry)
                        .unwrap()
                        .value
                        .as_u32()
                        .unwrap();
                    let mut buffer = vec![0u8; buffer_size as usize];
                    dng_clone.read_blob_to_buffer(&entry, &mut buffer).unwrap();
                    OpenOptions::new()
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .open(path.clone())
                        .unwrap()
                        .write(&buffer)
                        .unwrap();
                    Some(format!(
                        "file://{}",
                        path.strip_prefix(dir_clone.clone())
                            .unwrap()
                            .to_str()
                            .unwrap()
                    ))
                } else {
                    None
                }
            })),
        };

        let ifd_yaml = yaml_dumper.dump_ifd(&dng.get_ifd0());
        OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(dir.join("ifds.yml"))
            .unwrap()
            .write(ifd_yaml.as_bytes())
            .unwrap();
    } else {
        let yaml_dumper = YamlDumper {
            dump_rational_as_float: args.dump_rational_as_float,
            visitor: None,
        };
        let ifd_yaml = yaml_dumper.dump_ifd(&dng.get_ifd0());
        print!("{ifd_yaml}")
    }
}
