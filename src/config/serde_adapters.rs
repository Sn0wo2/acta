pub(crate) mod filter {
    use serde::{Deserialize as _, Deserializer, Serializer};
    use tracing_subscriber::EnvFilter;

    pub(crate) fn serialize<S: Serializer>(
        filter: &EnvFilter,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.collect_str(filter)
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<EnvFilter, D::Error> {
        let directive = String::deserialize(deserializer)?;
        EnvFilter::try_new(directive).map_err(serde::de::Error::custom)
    }
}

pub(crate) mod levels {
    use serde::{Deserialize as _, Deserializer, Serializer};
    use tracing::Level;

    pub(crate) fn serialize<S: Serializer>(
        levels: &[Level],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(levels.iter().map(Level::as_str))
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Vec<Level>, D::Error> {
        Vec::<String>::deserialize(deserializer)?
            .iter()
            .map(|s| s.trim().parse::<Level>().map_err(serde::de::Error::custom))
            .collect()
    }
}

pub(crate) mod optional_levels {
    use serde::{Deserialize as _, Deserializer, Serialize as _, Serializer};
    use tracing::Level;

    pub(crate) fn serialize<S: Serializer>(
        levels: &Option<Vec<Level>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        levels
            .as_ref()
            .map(|levels| levels.iter().map(Level::as_str).collect::<Vec<_>>())
            .serialize(serializer)
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<Vec<Level>>, D::Error> {
        Option::<Vec<String>>::deserialize(deserializer)?
            .map(|levels| {
                levels
                    .iter()
                    .map(|level| {
                        level
                            .trim()
                            .parse::<Level>()
                            .map_err(serde::de::Error::custom)
                    })
                    .collect()
            })
            .transpose()
    }
}
