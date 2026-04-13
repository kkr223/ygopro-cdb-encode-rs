# ygopro-cdb-encode-rs

[![Crates.io](https://img.shields.io/crates/v/ygopro-cdb-encode-rs.svg)](https://crates.io/crates/vgopro-cdb-encode-rs)
[![Documentation](https://docs.rs/ygopro-cdb-encode-rs/badge.svg)](https://docs.rs/ygopro-cdb-encode-rs)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A high-performance Rust library for encoding, decoding, and querying YGOPro `.cdb` (SQLite) databases. It aims to achieve parity with the upstream JS/TS project [`purerosefallen/ygopro-cdb-encode`](https://github.com/purerosefallen/ygopro-cdb-encode) while providing a idiomatic Rust API.

[中文说明](#中文说明)

---

## Features

- **Standard I/O**: Open from path/bytes or create new databases.
- **Semantic Mapping**: Automatically handles YGOPro-specific logic:
    - **Link Monsters**: Maps database `def` to `link_marker` and ensures `defense` is zero.
    - **Pendulum**: Correctly packs/unpacks scales into/from the `level` field.
    - **Aliases**: Resolves name/rule-code chains (e.g., alternative artworks).
- **Flexible Queries**:
    - **Raw SQL**: Query using optimized `WHERE` clauses with named parameters.
    - **Filter DSL**: A type-safe DSL for complex logical filtering (AND, OR, NOT, Bitwise race/attribute/type).
- **Dynamic Fields**: Supports "virtual" fields like `lscale`, `rscale`, `linkMarker`, and `ruleCode`.
- **Performance**: Built on `rusqlite` with regex caching and batch processing.
- **Robustness**: Comprehensive test suite ensuring compatibility with YGOPro conventions.

---

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
ygopro-cdb-encode-rs = "0.1.0"
serde_json = "1.0" # Required for some query parameters
```

---

## Quick Start

### 1. Simple Read

```rust
use ygopro_cdb_encode_rs::YgoProCdb;
use serde_json::json;

fn main() -> ygopro_cdb_encode_rs::Result<()> {
    let cdb = YgoProCdb::from_path("cards.cdb")?;

    // Find a card by name using raw SQL semantics
    let card = cdb.query_raw_one_with(
        "texts.name = :name",
        vec![("name", json!("Blue-Eyes White Dragon"))],
    )?;

    if let Some(c) = card {
        println!("Found ID: {}, ATK: {}", c.code, c.attack);
    }
    Ok(())
}
```

### 2. Complex Filtering (DSL)

```rust
use ygopro_cdb_encode_rs::{YgoProCdb, FindFilter, FilterCondition, and, more_than_or_equal, has_bit};

fn filter_demo(cdb: &YgoProCdb) -> ygopro_cdb_encode_rs::Result<()> {
    let filter = FindFilter::new()
        .with("type", has_bit(0x4000000)) // TYPE_LINK
        .with("attack", more_than_or_equal(2500));

    let cards = cdb.find(&filter)?;
    println!("Found {} high-ATK Link monsters", cards.len());
    Ok(())
}
```

### 3. Create and Export

```rust
use ygopro_cdb_encode_rs::{YgoProCdb, CardDataEntry, CardDataEntryPartial};

fn main() -> ygopro_cdb_encode_rs::Result<()> {
    let mut cdb = YgoProCdb::new()?;

    let card = CardDataEntry::default().from_partial(CardDataEntryPartial {
        code: Some(12345678),
        name: Some("Custom Card".into()),
        attack: Some(3000),
        defense: Some(2500),
        level: Some(8),
        ..Default::default()
    });

    cdb.add_card(card)?;
    let bytes = cdb.export()?; // Returns Vec<u8> of the SQLite file
    std::fs::write("new_cards.cdb", bytes)?;
    Ok(())
}
```

---

## Field Mapping

| Database Column | `CardDataEntry` Field | Notes |
| :--- | :--- | :--- |
| `datas.id` | `code` | Primary key |
| `datas.alias` | `alias` / `rule_code` | Smartly resolved based on YGOPro rules |
| `datas.setcode`| `setcode` | `Vec<u16>` |
| `datas.atk` | `attack` | |
| `datas.def` | `defense` / `link_marker` | Semantic auto-split for Links |
| `datas.level` | `level` / `lscale` / `rscale` | Semantic auto-split for Pendulums |
| `texts.name` | `name` | |
| `texts.desc` | `desc` | |

---

<h2 id="中文说明">中文说明</h2>

这是一个高性能的 Rust 库，用于 YGOPro `.cdb` (SQLite) 数据库的编码、解码和查询。

### 核心特性
1. **语义对齐**：自动处理 Link 怪兽的 `link_marker` 和灵摆怪兽的刻度值。
2. **规则码解析**：完善的 alias 链解析。
3. **强大查询**：支持原生 SQL `WHERE` 子句和类型安全的 `FindFilter` DSL。
4. **虚拟字段**：可在查询中直接使用 `lscale`, `rscale`, `linkMarker`, `ruleCode` 等虚拟字段。
5. **高性能**：基于 `rusqlite`，内置正则缓存和导出策略。

### 使用场景
- 开发桌面版 YGOPro 数据库编辑器。
- 需要在 Rust 后端或 WebAssembly 中高性能处理卡片数据的场景。
- 自动化卡图生成工具或脚本系统。

---

## License

This project is licensed under the [MIT License](LICENSE).
