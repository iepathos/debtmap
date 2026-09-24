//! Internal map keys retain complete declaration identities and read legacy keys.
use crate::priority::call_graph::FunctionId;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;

const PREFIX: &str = "v2:";

fn legacy_id(key: &str) -> Option<FunctionId> {
    let (location, line) = key.rsplit_once(':')?;
    let separator = location.char_indices().find_map(|(position, ch)| {
        if ch != ':' {
            return None;
        }
        let drive_separator = position == 1 && location[position + 1..].starts_with(['/', '\\']);
        let namespace_separator =
            location[..position].ends_with(':') || location[position + 1..].starts_with(':');
        (!drive_separator && !namespace_separator).then_some(position)
    })?;
    Some(FunctionId::new(
        location[..separator].into(),
        location[separator + 1..].to_owned(),
        line.parse().ok()?,
    ))
}

pub(super) mod function_id_serde {
    use super::*;

    pub fn serialize<S: Serializer, V: Serialize>(
        map: &HashMap<FunctionId, V>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let values: Result<HashMap<_, _>, S::Error> = map
            .iter()
            .map(|(id, value)| {
                serde_json::to_string(id)
                    .map(|key| (format!("{PREFIX}{key}"), value))
                    .map_err(serde::ser::Error::custom)
            })
            .collect();
        values?.serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>, V: Deserialize<'de>>(
        deserializer: D,
    ) -> Result<HashMap<FunctionId, V>, D::Error> {
        HashMap::<String, V>::deserialize(deserializer)?
            .into_iter()
            .map(|(key, value)| {
                let id = match key.strip_prefix(PREFIX) {
                    Some(json) => serde_json::from_str(json).map_err(serde::de::Error::custom),
                    None => legacy_id(&key).ok_or_else(|| {
                        serde::de::Error::custom("invalid legacy function identity")
                    }),
                }?;
                Ok((id, value))
            })
            .collect()
    }
}

pub(super) mod function_id_tuple_serde {
    use super::*;

    pub fn serialize<S: Serializer, V: Serialize>(
        map: &HashMap<(FunctionId, FunctionId), V>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let values: Result<HashMap<_, _>, S::Error> = map
            .iter()
            .map(|(ids, value)| {
                serde_json::to_string(ids)
                    .map(|key| (format!("{PREFIX}{key}"), value))
                    .map_err(serde::ser::Error::custom)
            })
            .collect();
        values?.serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>, V: Deserialize<'de>>(
        deserializer: D,
    ) -> Result<HashMap<(FunctionId, FunctionId), V>, D::Error> {
        HashMap::<String, V>::deserialize(deserializer)?
            .into_iter()
            .map(|(key, value)| {
                let ids = match key.strip_prefix(PREFIX) {
                    Some(json) => serde_json::from_str(json).map_err(serde::de::Error::custom),
                    None => key
                        .split_once('|')
                        .and_then(|(left, right)| Some((legacy_id(left)?, legacy_id(right)?)))
                        .ok_or_else(|| {
                            serde::de::Error::custom("invalid legacy transformation identity")
                        }),
                }?;
                Ok((ids, value))
            })
            .collect()
    }
}
