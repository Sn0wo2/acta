use chrono::format::{Item, StrftimeItems};

pub(crate) fn parse_time_items(fmt: &str) -> Vec<Item<'static>> {
    StrftimeItems::new(fmt).map(Item::to_owned).collect()
}
