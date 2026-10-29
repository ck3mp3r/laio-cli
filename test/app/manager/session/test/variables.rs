#[test]
fn encode_variables_empty() {
    use crate::app::manager::session::manager::encode_variables;

    let result = encode_variables(&[]);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "");
}

#[test]
fn encode_variables_simple() {
    use crate::app::manager::session::manager::encode_variables;

    let variables = vec!["key1=value1".to_string(), "key2=value2".to_string()];

    let result = encode_variables(&variables);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "key1=value1&key2=value2");
}

#[test]
fn encode_variables_with_special_chars() {
    use crate::app::manager::session::manager::encode_variables;

    let variables = vec![
        "path=/home/user/my project".to_string(),
        "cmd=echo 'hello world'".to_string(),
        "url=https://example.com?foo=bar&baz=qux".to_string(),
    ];

    let result = encode_variables(&variables);
    assert!(result.is_ok());
    let encoded = result.unwrap();

    // Values should be percent-encoded
    assert!(encoded.contains("path=%2Fhome%2Fuser%2Fmy%20project"));
    assert!(encoded.contains("cmd=echo%20%27hello%20world%27"));
    assert!(encoded.contains("url=https%3A%2F%2Fexample.com%3Ffoo%3Dbar%26baz%3Dqux"));
}

#[test]
fn encode_variables_invalid_format() {
    use crate::app::manager::session::manager::encode_variables;

    let variables = vec!["invalid_var".to_string()];

    let result = encode_variables(&variables);
    assert!(result.is_err());
}

#[test]
fn decode_variables_empty() {
    use crate::app::manager::session::manager::decode_variables;

    let result = decode_variables("");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Vec::<String>::new());
}

#[test]
fn decode_variables_simple() {
    use crate::app::manager::session::manager::decode_variables;

    let encoded = "key1=value1&key2=value2";
    let result = decode_variables(encoded);

    assert!(result.is_ok());
    let decoded = result.unwrap();
    assert_eq!(decoded, vec!["key1=value1", "key2=value2"]);
}

#[test]
fn decode_variables_with_special_chars() {
    use crate::app::manager::session::manager::decode_variables;

    let encoded = "path=%2Fhome%2Fuser%2Fmy%20project&cmd=echo%20%27hello%20world%27&url=https%3A%2F%2Fexample.com%3Ffoo%3Dbar%26baz%3Dqux";
    let result = decode_variables(encoded);

    assert!(result.is_ok());
    let decoded = result.unwrap();
    assert_eq!(decoded.len(), 3);
    assert_eq!(decoded[0], "path=/home/user/my project");
    assert_eq!(decoded[1], "cmd=echo 'hello world'");
    assert_eq!(decoded[2], "url=https://example.com?foo=bar&baz=qux");
}

#[test]
fn decode_variables_invalid_format() {
    use crate::app::manager::session::manager::decode_variables;

    let encoded = "invalid_pair";
    let result = decode_variables(encoded);

    assert!(result.is_err());
}

#[test]
fn encode_decode_roundtrip() {
    use crate::app::manager::session::manager::{decode_variables, encode_variables};

    let original = vec![
        "session_name=myproject".to_string(),
        "path=/home/user/my project".to_string(),
        "env=production".to_string(),
        "cmd=echo 'test'".to_string(),
    ];

    let encoded = encode_variables(&original).expect("Encoding failed");
    let decoded = decode_variables(&encoded).expect("Decoding failed");

    assert_eq!(decoded, original);
}
