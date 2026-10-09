//! Encode provider path components without allowing caller-controlled origins.
use crate::Config;
use rom_map_core::{Error, Result};
use url::Url;
pub(crate) fn prepare(config: &Config, segments: &[&str], query: &[(&str, &str)]) -> Result<Url> {
    if segments.is_empty() || segments.len() > 32 || query.len() > 32 {
        return Err(Error::InvalidQuery);
    }
    for value in segments {
        if value.len() > 4096 {
            return Err(Error::TooLarge);
        }
        if value.is_empty() || matches!(*value, "." | "..") || value.chars().any(char::is_control) {
            return Err(Error::InvalidQuery);
        }
    }
    let mut url = config.endpoint.clone();
    url.path_segments_mut()
        .map_err(|_| Error::InvalidQuery)?
        .pop_if_empty()
        .extend(segments.iter().copied());
    for (field, value) in query {
        if field.len() > 128 || value.len() > 4096 {
            return Err(Error::TooLarge);
        }
        if field.is_empty()
            || field.chars().any(char::is_control)
            || value.chars().any(char::is_control)
            || config.key.as_ref().is_some_and(|(key, _)| key == field)
        {
            return Err(Error::InvalidQuery);
        }
        url.query_pairs_mut().append_pair(field, value);
    }
    if let Some((field, key)) = &config.key {
        url.query_pairs_mut().append_pair(field, key);
    }
    if url.as_str().len() > 8192 {
        return Err(Error::TooLarge);
    }
    Ok(url)
}
