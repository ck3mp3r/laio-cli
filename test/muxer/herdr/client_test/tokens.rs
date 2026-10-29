use std::collections::HashMap;

use miette::Result;

use super::client::{chunk_token, chunk_tokens, join_tokens, split_chunk_key};

#[test]
fn chunk_token_splits_values_longer_than_eighty_chars() -> Result<()> {
    let short = chunk_token("LAIO_VARS", &"a".repeat(80));
    assert_eq!(short, vec![("LAIO_VARS".to_string(), "a".repeat(80))]);

    let long = chunk_token("LAIO_VARS", &"a".repeat(81));
    assert_eq!(long.len(), 2);
    assert_eq!(long[0].0, "LAIO_VARS_1");
    assert_eq!(long[0].1.len(), 80);
    assert_eq!(long[1].0, "LAIO_VARS_2");
    assert_eq!(long[1].1.len(), 1);
    Ok(())
}

#[test]
fn chunk_tokens_orders_keys_and_counts_chunks() -> Result<()> {
    let tokens = HashMap::from([
        ("LAIO_VARS".to_string(), "a".repeat(81)),
        ("LAIO_CONFIG".to_string(), "/tmp/laio.yaml".to_string()),
    ]);

    let pairs = chunk_tokens(&tokens);
    let keys: Vec<&str> = pairs.iter().map(|(key, _)| key.as_str()).collect();

    assert_eq!(keys, vec!["LAIO_CONFIG", "LAIO_VARS_1", "LAIO_VARS_2"]);
    Ok(())
}

#[test]
fn join_tokens_joins_chunks_in_numeric_order() -> Result<()> {
    let joined = join_tokens(HashMap::from([
        ("LAIO_VARS_10".to_string(), "c".to_string()),
        ("LAIO_VARS_2".to_string(), "b".to_string()),
        ("LAIO_VARS_1".to_string(), "a".to_string()),
        ("LAIO_CONFIG".to_string(), "/tmp/laio.yaml".to_string()),
    ]));

    assert_eq!(joined.get("LAIO_VARS"), Some(&"abc".to_string()));
    assert_eq!(
        joined.get("LAIO_CONFIG"),
        Some(&"/tmp/laio.yaml".to_string())
    );
    Ok(())
}

#[test]
fn split_chunk_key_recognises_only_numeric_suffixes() -> Result<()> {
    assert_eq!(split_chunk_key("LAIO_VARS_1"), Some(("LAIO_VARS", 1)));
    assert_eq!(split_chunk_key("LAIO_VARS_10"), Some(("LAIO_VARS", 10)));
    assert_eq!(split_chunk_key("LAIO_VARS"), None);
    assert_eq!(split_chunk_key("LAIO_VARS_0"), None);
    assert_eq!(split_chunk_key("LAIO_VARS_x"), None);
    assert_eq!(split_chunk_key("_1"), None);
    Ok(())
}

#[test]
fn chunk_and_join_round_trip() -> Result<()> {
    let value = "x".repeat(200);
    let pairs = chunk_token("LAIO_VARS", &value);
    let joined = join_tokens(pairs.into_iter().collect());

    assert_eq!(joined.get("LAIO_VARS"), Some(&value));
    Ok(())
}
