use json::JsonValue;
use std::env;
use std::fmt::Write as FmtWrite;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;

fn main() {
    let mut ifd_code = String::new();
    ifd_code += &parse_ifd_file("src/tags/ifd.json", "ifd");
    ifd_code += &parse_ifd_file("src/tags/exif.json", "exif");
    ifd_code += &parse_ifd_file("src/tags/gps_info.json", "gps_info");

    let out_dir = env::var("OUT_DIR").unwrap();
    let path = Path::new(&out_dir).join("ifd_data.rs");
    let mut f = File::create(path).unwrap();
    f.write_all(ifd_code.as_bytes()).unwrap();

    println!("cargo:rustc-cfg=has_generated_feature");
}

fn parse_ifd_file(path: &str, name: &str) -> String {
    println!("cargo:rerun-if-changed={path}");
    let contents = fs::read_to_string(path).expect("Unable to read file");
    let mut json: JsonValue = json::parse(&contents).expect("Unable to parse JSON");
    let entries: Vec<_> = json
        .members_mut()
        .map(|entry| parse_ifd_field_descriptor(entry.take()))
        .collect();
    let definitions: String = entries.iter().map(|(_, code)| code.to_string()).collect();
    let arr_contents: String = entries.iter().fold(String::new(), |mut output, (name, _)| {
        let _ = write!(output, "{name}, ");
        output
    });
    let len = entries.len();
    format!("
        /// Tags contained in the {name} namespace
        #[allow(non_upper_case_globals)]
        pub mod {name} {{
            #[allow(unused_imports)]
            use super::{{IfdFieldDescriptor, IfdValueType, IfdCount, IfdTypeInterpretation, IfdType}};
            pub(crate) const ALL: [IfdFieldDescriptor; {len}] = [{arr_contents}];
            {definitions}
        }}
    ")
}

fn parse_ifd_field_descriptor(mut json: JsonValue) -> (String, String) {
    let name = json.remove("name").take_string().unwrap();
    let tag = u16::from_str_radix(&json.remove("tag").take_string().unwrap()[2..], 16).unwrap();
    let dtype = parse_dtype(json.remove("dtype"));
    let interpretation = parse_interpretation(json.remove("interpretation"));
    let count = parse_count(json.remove("count"));
    let description = json.remove("description").take_string().unwrap();
    let long_description = json.remove("long_description").take_string().unwrap();
    let references = json.remove("references").take_string().unwrap();

    let code = format!(
        r#"
        IfdFieldDescriptor {{
            name: {name:?},
            tag: {tag},
            dtype: {dtype},
            interpretation: {interpretation},
            count: {count},
            description: {description:?},
            long_description: {long_description:?},
            references: {references:?},
        }}
    "#
    );
    let doc_description = doc_lines(description);
    let doc_long_description = doc_lines(long_description);
    let doc_references = doc_lines(references);
    let definition = format!(
        "
        {doc_description}
        ///
        {doc_long_description}
        ///
        /// references:  \\
        {doc_references}
        pub const {name}: IfdFieldDescriptor = {code};\n
    "
    );
    (name, definition)
}
fn doc_lines(lines: String) -> String {
    lines.lines().fold(String::new(), |mut out, s| {
        let _ = write!(out, "/// {s}");
        out
    })
}
fn parse_dtype(mut json: JsonValue) -> String {
    let entrys: String = json
        .members_mut()
        .map(|entry| parse_single_dtype(entry.take()) + ", ")
        .collect();
    format!("&[{entrys}]")
}
fn parse_single_dtype(json: JsonValue) -> String {
    match json.as_str().unwrap() {
        "BYTE" => "IfdValueType::Byte".to_string(),
        "ASCII" => "IfdValueType::Ascii".to_string(),
        "SHORT" => "IfdValueType::Short".to_string(),
        "LONG" => "IfdValueType::Long".to_string(),
        "RATIONAL" => "IfdValueType::Rational".to_string(),
        "SBYTE" => "IfdValueType::SByte".to_string(),
        "UNDEFINED" => "IfdValueType::Undefined".to_string(),
        "SSHORT" => "IfdValueType::SShort".to_string(),
        "SLONG" => "IfdValueType::SLong".to_string(),
        "SRATIONAL" => "IfdValueType::SRational".to_string(),
        "FLOAT" => "IfdValueType::Float".to_string(),
        "DOUBLE" => "IfdValueType::Double".to_string(),
        _ => unreachable!(),
    }
}
fn parse_count(json: JsonValue) -> String {
    let str = json.as_str().unwrap();
    match str.parse::<u32>() {
        Ok(n) => format!("IfdCount::ConcreteValue({n})"),
        Err(_) => "IfdCount::N".to_string(),
    }
}
fn parse_interpretation(mut json: JsonValue) -> String {
    let kind = json.remove("kind").take_string().unwrap();
    match kind.as_str() {
        "ENUMERATED" => {
            let values = parse_reverse_map(json.remove("values"));
            format!("IfdTypeInterpretation::Enumerated {{ values: {values} }}")
        }
        "BITFLAGS" => {
            let values = parse_reverse_map(json.remove("values"));
            format!("IfdTypeInterpretation::Bitflags {{ values: {values} }}")
        }
        "CFAPATTERN" => "IfdTypeInterpretation::CfaPattern".to_string(),
        "IFDOFFSET" => {
            let ifd_type = parse_ifd_type(json.remove("ifd_type"));
            format!("IfdTypeInterpretation::IfdOffset {{ ifd_type: {ifd_type} }}")
        }
        "OFFSETS" => {
            let lengths = json.remove("lengths").take_string().unwrap();
            format!("IfdTypeInterpretation::Offsets {{ lengths: &{lengths} }}")
        }
        "LENGTHS" => "IfdTypeInterpretation::Lengths".to_string(),
        "BLOB" => "IfdTypeInterpretation::Blob".to_string(),
        _ => "IfdTypeInterpretation::Default".to_string(),
    }
}
fn parse_ifd_type(json: JsonValue) -> String {
    match json.as_str().unwrap() {
        "IFD" => "IfdType::Ifd".to_string(),
        "EXIF" => "IfdType::Exif".to_string(),
        "GPSINFO" => "IfdType::GpsInfo".to_string(),
        _ => unreachable!(),
    }
}
fn parse_reverse_map(json: JsonValue) -> String {
    let entries: String = json.entries().fold(String::new(), |mut output, (k, v)| {
        let _ = write!(
            output,
            r#"({}, "{}"), "#,
            v.as_str().unwrap().replace("bit ", ""),
            k
        );
        output
    });
    format!("&[{entries}]")
}
