use std::{cell::RefCell, collections::HashMap, fs, path::Path};

use regex::Regex;
use rusqlite::{Connection, Statement, functions::FunctionFlags};
use serde_json::Value as JsonValue;
use tempfile::NamedTempFile;

use crate::{
    error::{CdbError, Result},
    filter::{FilterCondition, FilterValue, FindFilter},
    model::{CardDataEntry, TYPE_LINK, decode_setcode, normalize_alias_rule},
};

const CREATE_TABLE_STMT: &str = concat!(
    "CREATE TABLE IF NOT EXISTS datas(",
    "id integer primary key,",
    "ot integer,",
    "alias integer,",
    "setcode integer,",
    "type integer,",
    "atk integer,",
    "def integer,",
    "level integer,",
    "race integer,",
    "attribute integer,",
    "category integer",
    ");",
    "CREATE TABLE IF NOT EXISTS texts(",
    "id integer primary key,",
    "name text,",
    "desc text,",
    "str1 text,",
    "str2 text,",
    "str3 text,",
    "str4 text,",
    "str5 text,",
    "str6 text,",
    "str7 text,",
    "str8 text,",
    "str9 text,",
    "str10 text,",
    "str11 text,",
    "str12 text,",
    "str13 text,",
    "str14 text,",
    "str15 text,",
    "str16 text",
    ");"
);
const INSERT_EMPTY_TEXTS_FROM_DATAS_STMT: &str = concat!(
    "INSERT OR IGNORE INTO texts(",
    "id,name,desc,str1,str2,str3,str4,str5,str6,str7,str8,str9,str10,str11,str12,str13,str14,str15,str16",
    ") ",
    "SELECT datas.id,'' AS name,'' AS desc,'' AS str1,'' AS str2,'' AS str3,'' AS str4,'' AS str5,'' AS str6,'' AS str7,'' AS str8,'' AS str9,'' AS str10,'' AS str11,'' AS str12,'' AS str13,'' AS str14,'' AS str15,'' AS str16 ",
    "FROM datas"
);
const SELECT_CARD_COLUMNS: &str = concat!(
    "SELECT datas.id, datas.ot, datas.alias, datas.setcode, datas.type, datas.atk, datas.def, datas.level, datas.race, datas.attribute, datas.category,",
    " texts.name, texts.desc, texts.str1, texts.str2, texts.str3, texts.str4, texts.str5, texts.str6, texts.str7, texts.str8,",
    " texts.str9, texts.str10, texts.str11, texts.str12, texts.str13, texts.str14, texts.str15, texts.str16 ",
    "FROM datas INNER JOIN texts ON datas.id = texts.id"
);
const SELECT_CARD_COLUMNS_NO_TEXTS: &str = concat!(
    "SELECT datas.id, datas.ot, datas.alias, datas.setcode, datas.type, datas.atk, datas.def, datas.level, datas.race, datas.attribute, datas.category,",
    " '' AS name, '' AS desc, '' AS str1, '' AS str2, '' AS str3, '' AS str4, '' AS str5, '' AS str6, '' AS str7, '' AS str8,",
    " '' AS str9, '' AS str10, '' AS str11, '' AS str12, '' AS str13, '' AS str14, '' AS str15, '' AS str16 ",
    "FROM datas"
);
const INSERT_DATAS_STMT: &str = "INSERT OR REPLACE INTO datas(id,ot,alias,setcode,type,atk,def,level,race,attribute,category) VALUES (?,?,?,?,?,?,?,?,?,?,?)";
const INSERT_TEXTS_STMT: &str = concat!(
    "INSERT OR REPLACE INTO texts(",
    "id,name,desc,str1,str2,str3,str4,str5,str6,str7,str8,str9,str10,str11,str12,str13,str14,str15,str16",
    ") VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)"
);

#[derive(Debug)]
pub struct YgoProCdb {
    temp_file: NamedTempFile,
    conn: Connection,
    no_texts: bool,
}

#[derive(Debug, Clone, Default)]
struct SqlBuildContext {
    params: HashMap<String, JsonValue>,
    counter: usize,
}

impl SqlBuildContext {
    fn next_param(&mut self, value: JsonValue) -> String {
        let key = format!("p{}", self.counter);
        self.counter += 1;
        self.params.insert(key.clone(), value);
        format!(":{key}")
    }
}

impl YgoProCdb {
    pub fn new() -> Result<Self> {
        let temp_file = NamedTempFile::new()?;
        let conn = open_connection(temp_file.path())?;
        Ok(Self {
            temp_file,
            conn,
            no_texts: false,
        })
    }

    pub fn from_bytes(bytes: impl AsRef<[u8]>) -> Result<Self> {
        let temp_file = NamedTempFile::new()?;
        fs::write(temp_file.path(), bytes)?;
        let conn = open_connection(temp_file.path())?;
        Ok(Self {
            temp_file,
            conn,
            no_texts: false,
        })
    }

    pub fn from_path(path: impl AsRef<Path>) -> Result<Self> {
        Self::from_bytes(fs::read(path)?)
    }

    pub fn export(&self) -> Result<Vec<u8>> {
        self.conn.execute_batch("PRAGMA optimize;")?;
        Ok(fs::read(self.temp_file.path())?)
    }

    pub fn find_all(&self) -> Result<Vec<CardDataEntry>> {
        query_cards(
            &self.conn,
            "1=1 ORDER BY datas.id",
            &HashMap::new(),
            self.no_texts,
        )
    }

    pub fn query_raw(
        &self,
        where_clause: &str,
        params: &HashMap<String, JsonValue>,
    ) -> Result<Vec<CardDataEntry>> {
        let mut cards = query_cards(&self.conn, where_clause, params, self.no_texts)?;
        self.resolve_rule_codes(&mut cards)?;
        Ok(cards)
    }

    pub fn query_raw_with<I, K>(
        &self,
        where_clause: &str,
        params: I,
    ) -> Result<Vec<CardDataEntry>>
    where
        I: IntoIterator<Item = (K, JsonValue)>,
        K: Into<String>,
    {
        let params = collect_json_params(params);
        self.query_raw(where_clause, &params)
    }

    pub fn query_raw_one(
        &self,
        where_clause: &str,
        params: &HashMap<String, JsonValue>,
    ) -> Result<Option<CardDataEntry>> {
        let mut rows = self.query_raw(where_clause, params)?;
        Ok(rows.drain(..).next())
    }

    pub fn query_raw_one_with<I, K>(
        &self,
        where_clause: &str,
        params: I,
    ) -> Result<Option<CardDataEntry>>
    where
        I: IntoIterator<Item = (K, JsonValue)>,
        K: Into<String>,
    {
        let params = collect_json_params(params);
        self.query_raw_one(where_clause, &params)
    }

    pub fn find(&self, filter: &FindFilter) -> Result<Vec<CardDataEntry>> {
        let (sql, params) = build_filter_sql(filter, self.no_texts)?;
        self.query_raw(&sql, &params)
    }

    pub fn step(&self, filter: &FindFilter) -> Result<std::vec::IntoIter<CardDataEntry>> {
        Ok(self.find(filter)?.into_iter())
    }

    pub fn step_raw(
        &self,
        where_clause: &str,
        params: &HashMap<String, JsonValue>,
    ) -> Result<std::vec::IntoIter<CardDataEntry>> {
        Ok(self.query_raw(where_clause, params)?.into_iter())
    }

    pub fn step_raw_with<I, K>(
        &self,
        where_clause: &str,
        params: I,
    ) -> Result<std::vec::IntoIter<CardDataEntry>>
    where
        I: IntoIterator<Item = (K, JsonValue)>,
        K: Into<String>,
    {
        let params = collect_json_params(params);
        self.step_raw(where_clause, &params)
    }

    pub fn find_one(&self, filter: &FindFilter) -> Result<Option<CardDataEntry>> {
        let mut rows = self.find(filter)?;
        Ok(rows.drain(..).next())
    }

    pub fn find_by_id(&self, id: u32) -> Result<Option<CardDataEntry>> {
        let sql = format!(
            "{} WHERE datas.id = :id LIMIT 1",
            select_card_columns(self.no_texts)
        );
        let mut stmt = self.conn.prepare(&sql)?;
        if let Some(index) = stmt.parameter_index(":id")? {
            stmt.raw_bind_parameter(index, i64::from(id))?;
        }
        let mut rows = stmt.raw_query();
        let Some(row) = rows.next()? else {
            return Ok(None);
        };
        let mut card = card_from_row(row)?;
        self.resolve_rule_codes(std::slice::from_mut(&mut card))?;
        Ok(Some(card))
    }

    pub fn add_card(&mut self, card: CardDataEntry) -> Result<()> {
        upsert_cards(&mut self.conn, &[card], self.no_texts)
    }

    pub fn add_cards(&mut self, cards: &[CardDataEntry]) -> Result<()> {
        upsert_cards(&mut self.conn, cards, self.no_texts)
    }

    pub fn update_card(&mut self, card: CardDataEntry) -> Result<()> {
        upsert_cards(&mut self.conn, &[card], self.no_texts)
    }

    pub fn remove_card(&mut self, code: u32) -> Result<()> {
        delete_cards_by_id(&mut self.conn, &[code])
    }

    pub fn no_texts(&mut self, value: bool) -> Result<&mut Self> {
        self.no_texts = value;
        if value {
            self.conn.execute_batch("DROP TABLE IF EXISTS texts;")?;
        } else {
            self.conn.execute_batch(CREATE_TABLE_STMT)?;
            self.conn.execute_batch(INSERT_EMPTY_TEXTS_FROM_DATAS_STMT)?;
        }
        Ok(self)
    }

    fn resolve_rule_codes(&self, cards: &mut [CardDataEntry]) -> Result<()> {
        let mut alias_ids = Vec::new();
        for card in cards.iter() {
            if card.rule_code == 0 && card.alias != 0 && (card.type_ & crate::model::TYPE_TOKEN) == 0
            {
                alias_ids.push(card.alias);
            }
        }

        if alias_ids.is_empty() {
            return Ok(());
        }

        alias_ids.sort_unstable();
        alias_ids.dedup();

        let mut resolved = HashMap::new();
        for alias_id in alias_ids {
            let rule_code = self.resolve_rule_code_for_alias(alias_id)?;
            resolved.insert(alias_id, rule_code);
        }

        for card in cards.iter_mut() {
            if card.rule_code == 0 && card.alias != 0 {
                if let Some(rule_code) = resolved.get(&card.alias).copied().filter(|value| *value != 0)
                {
                    card.rule_code = rule_code;
                }
            }
        }

        Ok(())
    }

    fn resolve_rule_code_for_alias(&self, alias_id: u32) -> Result<u32> {
        let mut current_id = alias_id;
        let mut visited = std::collections::BTreeSet::new();

        loop {
            if !visited.insert(current_id) {
                return Ok(0);
            }

            let sql = "SELECT id, alias, type FROM datas WHERE id = :id LIMIT 1";
            let mut stmt = self.conn.prepare(sql)?;
            if let Some(index) = stmt.parameter_index(":id")? {
                stmt.raw_bind_parameter(index, i64::from(current_id))?;
            }

            let mut rows = stmt.raw_query();
            let Some(row) = rows.next()? else {
                return Ok(0);
            };

            let code = row.get::<_, i64>("id")? as u32;
            let alias = row.get::<_, i64>("alias")? as u32;
            let type_value = row.get::<_, i64>("type")? as u32;
            let (normalized_alias, rule_code) = normalize_alias_rule(code, alias, type_value);

            if rule_code != 0 {
                return Ok(rule_code);
            }

            if normalized_alias == 0 {
                return Ok(0);
            }

            current_id = normalized_alias;
        }
    }
}

fn collect_json_params<I, K>(params: I) -> HashMap<String, JsonValue>
where
    I: IntoIterator<Item = (K, JsonValue)>,
    K: Into<String>,
{
    params.into_iter().map(|(key, value)| (key.into(), value)).collect()
}

fn sanitize_clause(clause: &str) -> String {
    let trimmed = clause.trim();
    if trimmed.is_empty() {
        "1=1".to_string()
    } else {
        trimmed.to_string()
    }
}

fn select_card_columns(no_texts: bool) -> &'static str {
    if no_texts {
        SELECT_CARD_COLUMNS_NO_TEXTS
    } else {
        SELECT_CARD_COLUMNS
    }
}

fn ensure_clause_supported(where_clause: &str, no_texts: bool) -> Result<()> {
    if no_texts && where_clause.to_ascii_lowercase().contains("texts.") {
        return Err(CdbError::InvalidFilter(
            "texts table is not available in no_texts mode".to_string(),
        ));
    }
    Ok(())
}

fn open_connection(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path)?;
    let regex_cache: RefCell<HashMap<String, Regex>> = RefCell::new(HashMap::new());
    conn.create_scalar_function(
        "regexp",
        2,
        FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
        move |ctx| {
            let pattern = ctx.get::<String>(0)?;
            let input = ctx.get::<String>(1)?;
            if !regex_cache.borrow().contains_key(&pattern) {
                let compiled = Regex::new(&pattern)
                    .map_err(|err| rusqlite::Error::UserFunctionError(Box::new(err)))?;
                if regex_cache.borrow().len() >= 64 {
                    regex_cache.borrow_mut().clear();
                }
                regex_cache.borrow_mut().insert(pattern.clone(), compiled);
            }
            Ok(regex_cache.borrow()[&pattern].is_match(&input))
        },
    )?;
    conn.execute_batch(
        "PRAGMA journal_mode=DELETE; \
     PRAGMA synchronous=NORMAL; \
     PRAGMA temp_store=MEMORY; \
     PRAGMA foreign_keys=OFF;",
    )?;
    conn.execute_batch(CREATE_TABLE_STMT)?;
    conn.execute_batch(INSERT_EMPTY_TEXTS_FROM_DATAS_STMT)?;
    Ok(conn)
}

fn bind_json_params(stmt: &mut Statement<'_>, params: &HashMap<String, JsonValue>) -> Result<()> {
    for (key, value) in params {
        let parameter_name = if key.starts_with(':') || key.starts_with('@') || key.starts_with('$')
        {
            key.clone()
        } else {
            format!(":{key}")
        };

        let Some(index) = stmt.parameter_index(&parameter_name)? else {
            continue;
        };

        match value {
            JsonValue::Null => stmt.raw_bind_parameter(index, rusqlite::types::Null)?,
            JsonValue::Bool(boolean) => stmt.raw_bind_parameter(index, i64::from(*boolean))?,
            JsonValue::Number(number) => {
                if let Some(integer) = number.as_i64() {
                    stmt.raw_bind_parameter(index, integer)?;
                } else if let Some(unsigned) = number.as_u64() {
                    stmt.raw_bind_parameter(index, unsigned as i64)?;
                } else if let Some(float) = number.as_f64() {
                    stmt.raw_bind_parameter(index, float)?;
                }
            }
            JsonValue::String(text) => stmt.raw_bind_parameter(index, text.as_str())?,
            other => stmt.raw_bind_parameter(index, other.to_string())?,
        }
    }

    Ok(())
}

fn query_cards(
    conn: &Connection,
    where_clause: &str,
    params: &HashMap<String, JsonValue>,
    no_texts: bool,
) -> Result<Vec<CardDataEntry>> {
    ensure_clause_supported(where_clause, no_texts)?;
    let sql = format!(
        "{} WHERE {}",
        select_card_columns(no_texts),
        sanitize_clause(where_clause)
    );
    let mut stmt = conn.prepare(&sql)?;
    bind_json_params(&mut stmt, params)?;
    let mut rows = stmt.raw_query();
    let mut cards = Vec::new();

    while let Some(row) = rows.next()? {
        cards.push(card_from_row(row)?);
    }

    Ok(cards)
}

fn card_from_row(row: &rusqlite::Row<'_>) -> Result<CardDataEntry> {
    let code = row.get::<_, i64>("id")? as u32;
    let type_value = row.get::<_, i64>("type")? as u32;
    let mut defense = row.get::<_, i64>("def")? as i32;
    let mut link_marker = 0_u32;

    if (type_value & TYPE_LINK) != 0 {
        link_marker = defense.max(0) as u32;
        defense = 0;
    }

    let level_raw = row.get::<_, i64>("level")? as u32;
    let alias_raw = row.get::<_, i64>("alias")? as u32;
    let (alias, rule_code) = normalize_alias_rule(code, alias_raw, type_value);

    let mut strings = Vec::with_capacity(16);
    for index in 1..=16 {
        strings.push(
            row.get::<_, Option<String>>(format!("str{index}").as_str())?
                .unwrap_or_default(),
        );
    }

    Ok(CardDataEntry {
        code,
        alias,
        setcode: decode_setcode(row.get::<_, i64>("setcode")?),
        type_: type_value,
        attack: row.get::<_, i64>("atk")? as i32,
        defense,
        level: level_raw & 0xff,
        race: row.get::<_, i64>("race")? as u32,
        attribute: row.get::<_, i64>("attribute")? as u32,
        category: row.get::<_, i64>("category")? as u64,
        ot: row.get::<_, i64>("ot")? as u32,
        name: row.get::<_, Option<String>>("name")?.unwrap_or_default(),
        desc: row.get::<_, Option<String>>("desc")?.unwrap_or_default(),
        strings,
        lscale: (level_raw >> 24) & 0xff,
        rscale: (level_raw >> 16) & 0xff,
        link_marker,
        rule_code,
    })
}

fn write_card(
    stmt_datas: &mut Statement<'_>,
    stmt_texts: &mut Statement<'_>,
    card: &CardDataEntry,
) -> Result<()> {
    let mut strings = card.strings.clone();
    strings.resize(16, String::new());

    stmt_datas.execute(rusqlite::params![
        i64::from(card.code),
        i64::from(card.ot),
        i64::from(card.stored_alias()),
        card.packed_setcode(),
        i64::from(card.type_),
        i64::from(card.attack),
        i64::from(card.stored_defense()),
        i64::from(card.packed_level()),
        i64::from(card.race),
        i64::from(card.attribute),
        card.category as i64,
    ])?;

    stmt_texts.execute(rusqlite::params![
        i64::from(card.code),
        card.name,
        card.desc,
        strings[0],
        strings[1],
        strings[2],
        strings[3],
        strings[4],
        strings[5],
        strings[6],
        strings[7],
        strings[8],
        strings[9],
        strings[10],
        strings[11],
        strings[12],
        strings[13],
        strings[14],
        strings[15],
    ])?;

    Ok(())
}

fn upsert_cards(conn: &mut Connection, cards: &[CardDataEntry], no_texts: bool) -> Result<()> {
    let tx = conn.transaction()?;
    {
        let mut stmt_datas = tx.prepare(INSERT_DATAS_STMT)?;
        let mut stmt_texts = if no_texts {
            None
        } else {
            Some(tx.prepare(INSERT_TEXTS_STMT)?)
        };
        for card in cards {
            if let Some(stmt_texts) = stmt_texts.as_mut() {
                write_card(&mut stmt_datas, stmt_texts, card)?;
            } else {
                stmt_datas.execute(rusqlite::params![
                    i64::from(card.code),
                    i64::from(card.ot),
                    i64::from(card.stored_alias()),
                    card.packed_setcode(),
                    i64::from(card.type_),
                    i64::from(card.attack),
                    i64::from(card.stored_defense()),
                    i64::from(card.packed_level()),
                    i64::from(card.race),
                    i64::from(card.attribute),
                    card.category as i64,
                ])?;
            }
        }
    }
    tx.commit()?;
    Ok(())
}

fn delete_cards_by_id(conn: &mut Connection, card_ids: &[u32]) -> Result<()> {
    let tx = conn.transaction()?;
    for card_id in card_ids {
        tx.execute("DELETE FROM datas WHERE id = ?", [i64::from(*card_id)])?;
        tx.execute("DELETE FROM texts WHERE id = ?", [i64::from(*card_id)])
            .or_else(|err| match err {
                rusqlite::Error::SqliteFailure(_, Some(message))
                    if message.contains("no such table: texts") =>
                {
                    Ok(0)
                }
                other => Err(other),
            })?;
    }
    tx.commit()?;
    Ok(())
}

fn build_filter_sql(
    filter: &FindFilter,
    no_texts: bool,
) -> Result<(String, HashMap<String, JsonValue>)> {
    if filter.fields.is_empty() {
        return Ok(("1=1 ORDER BY datas.id".to_string(), HashMap::new()));
    }

    let mut ctx = SqlBuildContext::default();
    let mut clauses = Vec::with_capacity(filter.fields.len());

    for (field, condition) in &filter.fields {
        let expr = sql_expr_for_field(field, no_texts)?;
        let clause = build_condition_sql(expr.as_str(), condition, &mut ctx)?;
        clauses.push(clause);
    }

    Ok((
        format!("{} ORDER BY datas.id", clauses.join(" AND ")),
        ctx.params,
    ))
}

fn sql_expr_for_field(field: &str, no_texts: bool) -> Result<String> {
    let expr = match field {
        "code" => "datas.id",
        "ot" => "datas.ot",
        "alias" => {
            "CASE \
        WHEN datas.id = 5405695 THEN 0 \
        WHEN datas.alias != 0 AND (datas.type & 16384) = 0 \
          AND NOT (datas.alias < datas.id + 20 AND datas.id < datas.alias + 20) THEN 0 \
        ELSE datas.alias \
      END"
        }
        "setcode" => "datas.setcode",
        "type" => "datas.type",
        "attack" => "datas.atk",
        "rawDefense" => "datas.def",
        "defense" => "CASE WHEN (datas.type & 67108864) != 0 THEN NULL ELSE datas.def END",
        "linkMarker" => "CASE WHEN (datas.type & 67108864) != 0 THEN datas.def ELSE NULL END",
        "rawLevel" => "datas.level",
        "level" => "(datas.level & 255)",
        "lscale" => "((datas.level >> 24) & 255)",
        "rscale" => "((datas.level >> 16) & 255)",
        "race" => "datas.race",
        "attribute" => "datas.attribute",
        "category" => "datas.category",
        "id" => "datas.id",
        "atk" => "datas.atk",
        "def" => "datas.def",
        "name" => "texts.name",
        "desc" => "texts.desc",
        "str1" => "texts.str1",
        "str2" => "texts.str2",
        "str3" => "texts.str3",
        "str4" => "texts.str4",
        "str5" => "texts.str5",
        "str6" => "texts.str6",
        "str7" => "texts.str7",
        "str8" => "texts.str8",
        "str9" => "texts.str9",
        "str10" => "texts.str10",
        "str11" => "texts.str11",
        "str12" => "texts.str12",
        "str13" => "texts.str13",
        "str14" => "texts.str14",
        "str15" => "texts.str15",
        "str16" => "texts.str16",
        "ruleCode" => {
            "CASE \
        WHEN datas.id = 5405695 THEN datas.alias \
        WHEN datas.alias != 0 AND (datas.type & 16384) = 0 \
          AND NOT (datas.alias < datas.id + 20 AND datas.id < datas.alias + 20) THEN datas.alias \
        ELSE 0 \
      END"
        }
        other => {
            return Err(CdbError::InvalidFilter(format!(
                "unsupported filter field `{other}`"
            )));
        }
    };

    if no_texts && expr.starts_with("texts.") {
        return Err(CdbError::InvalidFilter(format!(
            "text field `{field}` is not available in no_texts mode"
        )));
    }

    Ok(expr.to_string())
}

fn build_condition_sql(
    field_expr: &str,
    condition: &FilterCondition,
    ctx: &mut SqlBuildContext,
) -> Result<String> {
    let clause = match condition {
        FilterCondition::Eq(FilterValue::Null) => format!("({field_expr} IS NULL)"),
        FilterCondition::Eq(value) => {
            let param = ctx.next_param(value.to_json());
            format!("({field_expr} = {param})")
        }
        FilterCondition::NotEq(FilterValue::Null) => {
            format!("({field_expr} IS NOT NULL)")
        }
        FilterCondition::NotEq(value) => {
            let param = ctx.next_param(value.to_json());
            format!("({field_expr} != {param})")
        }
        FilterCondition::LessThan(value) => {
            let param = ctx.next_param(value.to_json());
            format!("({field_expr} < {param})")
        }
        FilterCondition::LessThanOrEqual(value) => {
            let param = ctx.next_param(value.to_json());
            format!("({field_expr} <= {param})")
        }
        FilterCondition::MoreThan(value) => {
            let param = ctx.next_param(value.to_json());
            format!("({field_expr} > {param})")
        }
        FilterCondition::MoreThanOrEqual(value) => {
            let param = ctx.next_param(value.to_json());
            format!("({field_expr} >= {param})")
        }
        FilterCondition::HasBit(value) => {
            let param = ctx.next_param(JsonValue::from(*value));
            format!("(({field_expr} & {param}) != 0)")
        }
        FilterCondition::HasAllBits(value) => {
            let param = ctx.next_param(JsonValue::from(*value));
            format!("(({field_expr} & {param}) = {param})")
        }
        FilterCondition::And(list) => {
            let clauses = list
                .iter()
                .map(|item| build_condition_sql(field_expr, item, ctx))
                .collect::<Result<Vec<_>>>()?;
            format!("({})", clauses.join(" AND "))
        }
        FilterCondition::Or(list) => {
            let clauses = list
                .iter()
                .map(|item| build_condition_sql(field_expr, item, ctx))
                .collect::<Result<Vec<_>>>()?;
            format!("({})", clauses.join(" OR "))
        }
        FilterCondition::Not(inner) => {
            format!("(NOT {})", build_condition_sql(field_expr, inner, ctx)?)
        }
    };

    Ok(clause)
}
