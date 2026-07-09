use crate::dar_manifest::DarManifest;

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("header name too long")]
    NameTooLong,
    #[error("bad format (expected 'daml-lf')")]
    BadFormat,
    #[error("bad encryption (expected 'non-encrypted')")]
    BadEncryption,
    #[error("unexpected continuation line")]
    UnexpectedContinuation,
    #[error(transparent)]
    IoError(std::io::Error),
}

pub fn parse_manifest(source: &str) -> Result<DarManifest, ParseError> {
    let mut manifest = DarManifest {
        version: None,
        created_by: None,
        name: None,
        sdk_version: None,
        main_dalf: String::new(),
        dalfs: Vec::new(),
    };

    let mut current_header: Option<(String, String)> = None;
    let mut dalfs = String::new();

    for line in source.lines() {
        if let Some(cont_line) = line.strip_prefix(' ') {
            // continuation line
            let ch = current_header
                .as_mut()
                .ok_or(ParseError::UnexpectedContinuation)?;
            ch.1.push_str(cont_line);
        } else {
            // regular line
            if let Some(ch) = current_header {
                match ch.0.as_str() {
                    "Manifest-Version" => {
                        manifest.version = Some(ch.1);
                    }
                    "Created-By" => {
                        manifest.created_by = Some(ch.1);
                    }
                    "Name" => {
                        manifest.name = Some(ch.1);
                    }
                    "Sdk-Version" => {
                        manifest.sdk_version = Some(ch.1);
                    }
                    "Main-Dalf" => {
                        manifest.main_dalf = ch.1;
                    }
                    "Dalfs" => {
                        dalfs.push_str(&ch.1);
                    }
                    "Format" => {
                        if ch.1 != "daml-lf" {
                            return Err(ParseError::BadFormat);
                        }
                    }
                    "Encryption" if ch.1 != "non-encrypted" => {
                        return Err(ParseError::BadEncryption);
                    }
                    _ => {}
                }
            }

            if line.is_empty() {
                break;
            }

            let (name, value) = line.split_once(": ").ok_or(ParseError::NameTooLong)?;
            current_header = Some((name.to_owned(), value.to_owned()));
        }
    }

    manifest.dalfs = dalfs.split(",").map(|s| s.trim().to_owned()).collect();

    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;

    use pretty_assertions::assert_eq;

    const EXAMPLE_MANIFEST_STRING: &str = r#"Manifest-Version: 1.0
Created-By: damlc
Name: my-contracts-0.1.0
Sdk-Version: 0.0.0
Main-Dalf: my-contracts-0.1.0-8b2fe88417a032ed26641c165dbd812428a6010efbc
 76f6cdb69a46bd4d23d5e/my-contracts-0.1.0-8b2fe88417a032ed26641c165dbd812
 428a6010efbc76f6cdb69a46bd4d23d5e.dalf
Dalfs: my-contracts-0.1.0-8b2fe88417a032ed26641c165dbd812428a6010efbc76f6
 cdb69a46bd4d23d5e/my-contracts-0.1.0-8b2fe88417a032ed26641c165dbd812428a
 6010efbc76f6cdb69a46bd4d23d5e.dalf, my-contracts-0.1.0-8b2fe88417a032ed2
 6641c165dbd812428a6010efbc76f6cdb69a46bd4d23d5e/daml-prim-c017ae1b1a3e
 49ed89f06d5ecfab284e4b98ff90e35840a2a8d09b354ae78cdd.dalf
Format: daml-lf
Encryption: non-encrypted

"#;

    #[test]
    fn test_parse_manifest() {
        let expected = DarManifest {
            version: Some("1.0".to_string()),
            created_by: Some("damlc".to_string()),
            name: Some("my-contracts-0.1.0".to_string()),
            sdk_version: Some("0.0.0".to_string()),
            main_dalf:
                "my-contracts-0.1.0-8b2fe88417a032ed26641c165dbd812428a6010efbc76f6cdb69a46bd4d23d5e/\
                 my-contracts-0.1.0-8b2fe88417a032ed26641c165dbd812428a6010efbc76f6cdb69a46bd4d23d5e.\
                 dalf"
                    .to_string(),
            dalfs: vec![
                "my-contracts-0.1.0-8b2fe88417a032ed26641c165dbd812428a6010efbc76f6cdb69a46bd4d23d5e/\
                 my-contracts-0.1.0-8b2fe88417a032ed26641c165dbd812428a6010efbc76f6cdb69a46bd4d23d5e.\
                 dalf"
                    .to_string(),
                "my-contracts-0.1.0-8b2fe88417a032ed26641c165dbd812428a6010efbc76f6cdb69a46bd4d23d5e/\
                 daml-prim-c017ae1b1a3e49ed89f06d5ecfab284e4b98ff90e35840a2a8d09b354ae78cdd.dalf"
                    .to_string(),
            ],
        };

        let manifest = parse_manifest(EXAMPLE_MANIFEST_STRING).expect("manifest parsed");
        assert_eq!(manifest, expected);
    }
}
