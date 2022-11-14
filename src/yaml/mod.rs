pub mod dumper;
pub mod parser;

#[cfg(test)]
mod tests {
    use crate::yaml::dumper::IfdYamlDumper;
    use crate::yaml::parser::IfdYamlParser;
    use std::fs;

    #[test]
    fn test_axiom_beta_sim_yaml() {
        parse_serialize_parse("src/yaml/testdata/axiom_beta_simulated.yml")
    }

    #[test]
    fn test_axiom_recorder_dng_converter_yml() {
        parse_serialize_parse("src/yaml/testdata/axiom_recorder_dng_converter.yml")
    }

    #[test]
    fn test_pentax_k30_yml() {
        parse_serialize_parse("src/yaml/testdata/pentax_k30.yml")
    }

    #[test]
    fn test_pentax_k30_dng_converter() {
        parse_serialize_parse("src/yaml/testdata/pentax_k30_dng_converter.yml")
    }

    fn parse_serialize_parse(path: &str) {
        let data = fs::read_to_string(path).expect("Unable to read file");
        let parsed = IfdYamlParser::parse_from_str(&data).unwrap();
        let serialized = IfdYamlDumper::default().dump_ifd(&parsed);
        let _parsed_second = IfdYamlParser::parse_from_str(&serialized).unwrap();
    }
}
