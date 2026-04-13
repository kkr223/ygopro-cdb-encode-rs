# ygopro-cdb-encode-rs

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A Rust library for reading, writing, querying, and transforming YGOPro `.cdb` (SQLite) databases.

It aims to stay compatible with the upstream JS/TS project [`purerosefallen/ygopro-cdb-encode`](https://github.com/purerosefallen/ygopro-cdb-encode), while providing an idiomatic Rust API for desktop apps, backend services, automation tools, and future renderer pipelines.

[中文说明](#中文说明)

---

## Features

- Open databases from memory, from disk, or create new ones in place.
- Read through ergonomic semantic fields instead of raw packed CDB columns.
- Query with either:
  - raw SQL `WHERE` clauses plus named parameters
  - a typed `FindFilter` DSL with logical composition and bitwise conditions
- Page and count query results without rebuilding the query yourself.
- Fetch cards by single ID or batched IDs.
- Insert, update, delete, batch-write, and undo a modify/delete transaction.
- Export to bytes or directly copy the current database to a target file.
- Optional `no_texts` mode for datas-only workflows.
- Built on `rusqlite` with regex support, cached regex compilation, and YGOPro-aware field normalization.

---

## Semantic Mapping

`CardDataEntry` exposes YGOPro data in a friendlier shape:

- Link monsters:
  - database `datas.def` is mapped to `link_marker`
  - exposed `defense` becomes `0`
- Pendulum monsters:
  - packed `datas.level` is split into `level`, `lscale`, and `rscale`
- Alias and rule handling:
  - `alias` and `rule_code` are normalized according to YGOPro conventions
  - alias chains can be resolved when loading/querying cards
- Setcodes:
  - packed `datas.setcode` is decoded into `Vec<u16>`

---

## Usage

Add this to your `Cargo.toml` as a local path dependency:

```toml
[dependencies]
ygopro-cdb-encode-rs = { path = "../ygopro-cdb-encode-rs" }
serde_json = "1.0"
```

`serde_json` is needed for raw SQL parameter maps.

---

## Quick Start

### 1. Read a database

```rust
use serde_json::json;
use ygopro_cdb_encode_rs::YgoProCdb;

fn main() -> ygopro_cdb_encode_rs::Result<()> {
    let cdb = YgoProCdb::from_path("cards.cdb")?;

    let card = cdb.query_raw_one_with(
        "texts.name = :name",
        vec![("name", json!("Blue-Eyes White Dragon"))],
    )?;

    if let Some(card) = card {
        println!("Found {} with ATK {}", card.code, card.attack);
    }

    Ok(())
}
```

### 2. Page and count results

```rust
use serde_json::json;
use std::collections::HashMap;
use ygopro_cdb_encode_rs::YgoProCdb;

fn main() -> ygopro_cdb_encode_rs::Result<()> {
    let cdb = YgoProCdb::from_path("cards.cdb")?;

    let params = HashMap::from([
        ("min_atk".to_string(), json!(2000)),
    ]);

    let (cards, total) = cdb.query_raw_page(
        "datas.atk >= :min_atk",
        &params,
        1,
        50,
    )?;

    println!("Page size: {}, total matches: {}", cards.len(), total);
    Ok(())
}
```

### 3. Filter with the DSL

```rust
use ygopro_cdb_encode_rs::{
    FindFilter, TYPE_LINK, YgoProCdb, has_bit, more_than_or_equal,
};

fn filter_demo(cdb: &YgoProCdb) -> ygopro_cdb_encode_rs::Result<()> {
    let filter = FindFilter::new()
        .with("type", has_bit(TYPE_LINK.into()))
        .with("attack", more_than_or_equal(2500));

    let cards = cdb.find(&filter)?;
    println!("Found {} high-ATK Link monsters", cards.len());
    Ok(())
}
```

### 4. Create and write a database

```rust
use ygopro_cdb_encode_rs::{CardDataEntry, CardDataEntryPartial, YgoProCdb};

fn main() -> ygopro_cdb_encode_rs::Result<()> {
    let mut cdb = YgoProCdb::create_at_path("new_cards.cdb")?;

    let card = CardDataEntry::default().from_partial(CardDataEntryPartial {
        code: Some(12345678),
        name: Some("Custom Card".into()),
        attack: Some(3000),
        defense: Some(2500),
        level: Some(8),
        ..Default::default()
    });

    cdb.add_card(card)?;
    cdb.export_to_path("new_cards_copy.cdb")?;
    Ok(())
}
```

---

## API Overview

### Database lifecycle

- `YgoProCdb::new()`
  - Create a temporary working database.
- `YgoProCdb::from_bytes(bytes)`
  - Open from raw `.cdb` bytes.
- `YgoProCdb::from_path(path)`
  - Open a database by reading the file into an internal temporary copy.
- `YgoProCdb::from_path_direct(path)`
  - Open and operate on the database file directly.
- `YgoProCdb::create_at_path(path)`
  - Create a new empty database at a specific file path.
- `YgoProCdb::path()`
  - Get the underlying database file path currently used by the instance.

### Export

- `export() -> Result<Vec<u8>>`
- `export_to_path(path) -> Result<()>`

### Reading and lookup

- `find_all()`
- `find_one(filter)`
- `find(filter)`
- `step(filter)`
- `find_by_id(id)`
- `find_by_ids(ids)`

### Raw SQL querying

All raw SQL methods expect a `WHERE` clause, not a full `SELECT`.

- `query_raw(where_clause, params)`
- `query_raw_with(where_clause, iterable_params)`
- `query_raw_one(where_clause, params)`
- `query_raw_one_with(where_clause, iterable_params)`
- `query_raw_page(where_clause, params, page, page_size)`
- `count_raw(where_clause, params)`
- `count_raw_with(where_clause, iterable_params)`
- `step_raw(where_clause, params)`
- `step_raw_with(where_clause, iterable_params)`

Example:

```rust
use serde_json::json;
use std::collections::HashMap;
use ygopro_cdb_encode_rs::YgoProCdb;

fn main() -> ygopro_cdb_encode_rs::Result<()> {
    let cdb = YgoProCdb::from_path("cards.cdb")?;
    let params = HashMap::from([
        ("race".to_string(), json!(8192)),
        ("min_atk".to_string(), json!(1800)),
    ]);

    let cards = cdb.query_raw(
        "datas.race = :race AND datas.atk >= :min_atk ORDER BY datas.id",
        &params,
    )?;

    println!("Found {} cards", cards.len());
    Ok(())
}
```

### Writing

- `add_card(card)`
- `add_cards(cards)`
- `update_card(card)`
- `remove_card(code)`
- `remove_cards(codes)`
- `undo_modify(cards_to_restore, ids_to_delete)`

`undo_modify` is useful when an editor wants to restore previous cards and delete newly-created cards in one atomic transaction.

### Optional datas-only mode

- `no_texts(true)`
  - Drops the `texts` table from the current working database and restricts future queries to `datas`-only fields.
- `no_texts(false)`
  - Recreates the `texts` table and fills missing text rows.

This can be useful for lightweight structural workflows, but any query referencing `texts.*` will fail while `no_texts` is enabled.

---

## Filter DSL

The crate exports:

- `FindFilter`
- `FilterCondition`
- `FilterValue`
- helpers:
  - `and`
  - `or`
  - `not`
  - `less_than`
  - `less_than_or_equal`
  - `more_than`
  - `more_than_or_equal`
  - `has_bit`
  - `has_all_bits`

Typical fields include:

- raw columns:
  - `code`
  - `alias`
  - `attack`
  - `defense`
  - `level`
  - `race`
  - `attribute`
  - `type`
  - `ot`
  - `category`
  - `name`
  - `desc`
- semantic/virtual fields:
  - `lscale`
  - `rscale`
  - `linkMarker`
  - `ruleCode`
  - `setcode`

Example:

```rust
use ygopro_cdb_encode_rs::{
    FindFilter, TYPE_LINK, and, has_all_bits, more_than_or_equal,
};

let filter = FindFilter::new()
    .with("type", has_all_bits(TYPE_LINK.into()))
    .with("attack", more_than_or_equal(2000));
```

Note: this crate currently exports card type constants, but not named race/attribute constants yet. For race/attribute filters, pass the numeric values used by YGOPro.

---

## Data Types

### `CardDataEntry`

Primary full card model used for reads and writes.

Important fields:

- `code`
- `alias`
- `setcode: Vec<u16>`
- `type_` serialized as `type`
- `attack`
- `defense`
- `level`
- `race`
- `attribute`
- `category`
- `ot`
- `name`
- `desc`
- `strings`
- `lscale`
- `rscale`
- `link_marker` serialized as `linkMarker`
- `rule_code` serialized as `ruleCode`

### `CardDataEntryPartial`

Convenience partial model for constructing or patching cards via:

```rust
let card = CardDataEntry::default().from_partial(partial);
```

### Exported constants

- `TYPE_MONSTER`
- `TYPE_SPELL`
- `TYPE_TRAP`
- `TYPE_FUSION`
- `TYPE_RITUAL`
- `TYPE_SYNCHRO`
- `TYPE_XYZ`
- `TYPE_PENDULUM`
- `TYPE_LINK`
- `TYPE_TOKEN`
- `CARD_ARTWORK_VERSIONS_OFFSET`

---

## Field Mapping

| Database Column | `CardDataEntry` Field | Notes |
| :--- | :--- | :--- |
| `datas.id` | `code` | Primary key |
| `datas.alias` | `alias` / `rule_code` | Normalized with YGOPro alias/rule behavior |
| `datas.setcode` | `setcode` | Decoded as `Vec<u16>` |
| `datas.type` | `type_` | Serialized as `type` |
| `datas.atk` | `attack` | |
| `datas.def` | `defense` / `link_marker` | Link monsters expose arrows via `link_marker` |
| `datas.level` | `level` / `lscale` / `rscale` | Pendulum scales are unpacked |
| `texts.name` | `name` | |
| `texts.desc` | `desc` | |
| `texts.str1..str16` | `strings` | Resized to 16 slots on write |

---

## Error Type

All fallible operations return:

```rust
ygopro_cdb_encode_rs::Result<T>
```

Backed by:

- `CdbError::Database`
- `CdbError::Io`
- `CdbError::InvalidFilter`

---

<h2 id="中文说明">中文说明</h2>

这是一个用于 YGOPro `.cdb` 数据库读写、查询和转换的 Rust 库。

### 主要能力

1. 支持从字节、路径、直接文件句柄路径打开数据库，也支持原地创建新库。
2. 自动处理 YGOPro 语义字段：
   - Link 怪兽的 `def -> linkMarker`
   - 灵摆怪兽的 `level -> level/lscale/rscale`
   - alias / ruleCode 规范化与链式解析
   - packed `setcode` 解码
3. 支持两套查询方式：
   - 原生 SQL `WHERE` 子句
   - 类型安全的 `FindFilter` DSL
4. 新增了分页、计数和批量读取接口：
   - `query_raw_page`
   - `count_raw`
   - `find_by_ids`
   - `step_raw`
5. 写入侧支持批量增删改和原子撤销：
   - `add_cards`
   - `remove_cards`
   - `undo_modify`
6. 支持 `export_to_path`、`create_at_path`、`from_path_direct` 等更适合桌面编辑器/工作副本的接口。
7. 支持 `no_texts(true)` 进入仅 `datas` 表模式，适合轻量处理流程。

### 适合的场景

- 桌面版 CDB 编辑器
- Rust 后端批处理或自动化工具
- 卡片渲染前的数据预处理
- 与自定义工作副本、撤销/重做系统集成的应用

---

## License

This project is licensed under the [MIT License](LICENSE).
