use serde::{Deserialize, Serialize};

pub const TYPE_MONSTER: u32 = 0x1;
pub const TYPE_SPELL: u32 = 0x2;
pub const TYPE_TRAP: u32 = 0x4;
pub const TYPE_FUSION: u32 = 0x40;
pub const TYPE_RITUAL: u32 = 0x80;
pub const TYPE_TOKEN: u32 = 0x4000;
pub const TYPE_SYNCHRO: u32 = 0x2000;
pub const TYPE_XYZ: u32 = 0x800000;
pub const TYPE_PENDULUM: u32 = 0x1000000;
pub const TYPE_LINK: u32 = 0x4000000;
pub const CARD_ARTWORK_VERSIONS_OFFSET: u32 = 20;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CardDataEntry {
    pub code: u32,
    pub alias: u32,
    pub setcode: Vec<u16>,
    #[serde(rename = "type")]
    pub type_: u32,
    pub attack: i32,
    pub defense: i32,
    pub level: u32,
    pub race: u32,
    pub attribute: u32,
    pub category: u64,
    pub ot: u32,
    pub name: String,
    pub desc: String,
    #[serde(default)]
    pub strings: Vec<String>,
    pub lscale: u32,
    pub rscale: u32,
    pub link_marker: u32,
    pub rule_code: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CardDataEntryPartial {
    pub code: Option<u32>,
    pub alias: Option<u32>,
    pub setcode: Option<Vec<u16>>,
    #[serde(rename = "type")]
    pub type_: Option<u32>,
    pub attack: Option<i32>,
    pub defense: Option<i32>,
    pub level: Option<u32>,
    pub race: Option<u32>,
    pub attribute: Option<u32>,
    pub category: Option<u64>,
    pub ot: Option<u32>,
    pub name: Option<String>,
    pub desc: Option<String>,
    pub strings: Option<Vec<String>>,
    pub lscale: Option<u32>,
    pub rscale: Option<u32>,
    pub link_marker: Option<u32>,
    pub rule_code: Option<u32>,
}

impl CardDataEntry {
    pub fn from_partial(mut self, partial: CardDataEntryPartial) -> Self {
        if let Some(value) = partial.code {
            self.code = value;
        }
        if let Some(value) = partial.alias {
            self.alias = value;
        }
        if let Some(value) = partial.setcode {
            self.setcode = value;
        }
        if let Some(value) = partial.type_ {
            self.type_ = value;
        }
        if let Some(value) = partial.attack {
            self.attack = value;
        }
        if let Some(value) = partial.defense {
            self.defense = value;
        }
        if let Some(value) = partial.level {
            self.level = value;
        }
        if let Some(value) = partial.race {
            self.race = value;
        }
        if let Some(value) = partial.attribute {
            self.attribute = value;
        }
        if let Some(value) = partial.category {
            self.category = value;
        }
        if let Some(value) = partial.ot {
            self.ot = value;
        }
        if let Some(value) = partial.name {
            self.name = value;
        }
        if let Some(value) = partial.desc {
            self.desc = value;
        }
        if let Some(value) = partial.strings {
            let mut strings = value;
            strings.resize(16, String::new());
            self.strings = strings;
        }
        if let Some(value) = partial.lscale {
            self.lscale = value;
        }
        if let Some(value) = partial.rscale {
            self.rscale = value;
        }
        if let Some(value) = partial.link_marker {
            self.link_marker = value;
        }
        if let Some(value) = partial.rule_code {
            self.rule_code = value;
        }
        self
    }

    pub fn is_link(&self) -> bool {
        (self.type_ & TYPE_LINK) != 0
    }

    pub fn is_spell(&self) -> bool {
        (self.type_ & TYPE_SPELL) != 0
    }

    pub fn is_trap(&self) -> bool {
        (self.type_ & TYPE_TRAP) != 0
    }

    pub fn is_monster(&self) -> bool {
        (self.type_ & TYPE_MONSTER) != 0
    }

    pub fn is_pendulum(&self) -> bool {
        (self.type_ & TYPE_PENDULUM) != 0
    }

    pub fn packed_setcode(&self) -> i64 {
        let mut value = 0_u64;
        for (index, chunk) in self.setcode.iter().take(4).enumerate() {
            value |= (u64::from(*chunk) & 0xffff) << (index * 16);
        }
        value as i64
    }

    pub fn packed_level(&self) -> u32 {
        (self.level & 0xff) | ((self.rscale & 0xff) << 16) | ((self.lscale & 0xff) << 24)
    }

    pub fn stored_alias(&self) -> u32 {
        if self.alias != 0 {
            self.alias
        } else {
            self.rule_code
        }
    }

    pub fn stored_defense(&self) -> i32 {
        if self.is_link() {
            self.link_marker as i32
        } else {
            self.defense
        }
    }
}

pub fn decode_setcode(raw: i64) -> Vec<u16> {
    let mut value = raw as u64;
    let mut result = Vec::new();
    while value != 0 && result.len() < 4 {
        let chunk = (value & 0xffff) as u16;
        if chunk != 0 {
            result.push(chunk);
        }
        value >>= 16;
    }
    result
}

pub fn normalize_alias_rule(code: u32, alias: u32, type_value: u32) -> (u32, u32) {
    let mut normalized_alias = alias;
    let mut rule_code = 0_u32;

    if code == 5_405_695 {
        rule_code = normalized_alias;
        normalized_alias = 0;
    } else if normalized_alias != 0 && (type_value & TYPE_TOKEN) == 0 {
        let is_alternative = normalized_alias < code + CARD_ARTWORK_VERSIONS_OFFSET
            && code < normalized_alias + CARD_ARTWORK_VERSIONS_OFFSET;
        if !is_alternative {
            rule_code = normalized_alias;
            normalized_alias = 0;
        }
    }

    (normalized_alias, rule_code)
}
