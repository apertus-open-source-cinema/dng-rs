use clap::Parser;
use dng::byte_order_rw::ByteOrderWriter;
use dng::ifd::{IfdEntry, IfdValue};
use dng::ifd_tags::IfdTypeInterpretation;
use dng::yaml::IfdYamlDumper;
use dng::DngReader;
use dng::DngWriter;
use itertools::Itertools;
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
    let dng = Arc::new(DngReader::read(img_file).expect("Couldnt parse DNG file!"));

    let matrix_prettify_visitor = move |entry: IfdEntry| -> Option<String> {
        if entry
            .tag
            .get_known_name()
            .map_or(true, |name| !name.to_lowercase().contains("matrix"))
        {
            return None;
        }
        if let IfdValue::List(list) = entry.value {
            let dumper = IfdYamlDumper {
                dump_rational_as_float: args.dump_rational_as_float,
                visitor: None,
            };
            let wrapped_string = list
                .chunks(3)
                .map(|chunk| {
                    chunk
                        .iter()
                        .map(|entry| format!("{},", dumper.dump_ifd_value(entry)))
                        .join(" ")
                })
                .join("\n");
            return Some(format!(
                "[\n{}\n]",
                textwrap::indent(&*wrapped_string, "  ")
            ));
        }
        None
    };

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

        let extract_visitor = {
            let dir = dir.clone();
            let dng = dng.clone();
            let matrix_prettify_visitor = matrix_prettify_visitor.clone();
            move |entry: IfdEntry| -> Option<String> {
                if matches!(
                    entry.tag.get_known_type_interpretation(),
                    Some(IfdTypeInterpretation::Blob)
                ) {
                    let path = dir.join(entry.path.string_with_separator("_"));
                    let file = OpenOptions::new()
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .open(path.clone())
                        .unwrap();
                    DngWriter::write_primitive_value(
                        &entry.value,
                        &mut ByteOrderWriter::new(file, true),
                    )
                    .unwrap();
                    return Some(format!(
                        "file://{}",
                        path.strip_prefix(dir.clone()).unwrap().to_str().unwrap()
                    ));
                }
                if matches!(
                    entry.tag.get_known_type_interpretation(),
                    Some(IfdTypeInterpretation::Offsets { .. })
                ) && !matches!(entry.value, IfdValue::List(_))
                {
                    let path = dir.join(entry.path.string_with_separator("_"));
                    let buffer_size = dng
                        .needed_buffer_size_for_blob(&entry)
                        .unwrap()
                        .value
                        .as_u32()
                        .unwrap();
                    let mut buffer = vec![0u8; buffer_size as usize];
                    dng.read_blob_to_buffer(&entry, &mut buffer).unwrap();
                    OpenOptions::new()
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .open(path.clone())
                        .unwrap()
                        .write(&buffer)
                        .unwrap();
                    return Some(format!(
                        "file://{}",
                        path.strip_prefix(dir.clone()).unwrap().to_str().unwrap()
                    ));
                }
                matrix_prettify_visitor(entry)
            }
        };
        let yaml_dumper = IfdYamlDumper {
            dump_rational_as_float: args.dump_rational_as_float,
            visitor: Some(Arc::new(extract_visitor)),
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
        let yaml_dumper = IfdYamlDumper {
            dump_rational_as_float: args.dump_rational_as_float,
            visitor: Some(Arc::new(matrix_prettify_visitor)),
        };
        let ifd_yaml = yaml_dumper.dump_ifd(&dng.get_ifd0());
        print!("{ifd_yaml}")
    }
}
