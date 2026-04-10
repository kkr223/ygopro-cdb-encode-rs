# ygopro-cdb-encode-rs

Rust 版的 YGOPro `.cdb` 编码、解码与查询库。

当前实现目标是尽量对齐上游 JS/TS 项目 `purerosefallen/ygopro-cdb-encode` 的核心行为，同时提供更适合 Rust 使用的 API。

## 当前能力

- 读取现有 `.cdb`：`YgoProCdb::from_path` / `YgoProCdb::from_bytes`
- 创建空库并写回 `.cdb`：`YgoProCdb::new` / `add_card` / `add_cards` / `export`
- 统一卡片模型：`CardDataEntry`
- 便捷构造：`CardDataEntry::from_partial(CardDataEntryPartial)`
- 按 ID 查询：`find_by_id`
- 原始 SQL `WHERE` 查询：`query_raw` / `query_raw_one`
- 更方便的命名参数查询：`query_raw_with` / `query_raw_one_with` / `step_raw_with`
- Filter DSL 查询：`FindFilter`
- 上游风格运算符：`not` / `less_than` / `more_than` / `less_than_or_equal` / `more_than_or_equal` / `and` / `or` / `has_bit` / `has_all_bits`
- 虚拟字段查询：`code` / `level` / `rawLevel` / `rawDefense` / `defense` / `linkMarker` / `lscale` / `rscale` / `ruleCode`
- `no_texts` 模式，兼容缺少 `texts` 表的数据库
- Link 怪兽防御 / `link_marker`、异画 alias / `rule_code` 等 YGOPro 语义转换

## 安装

```toml
[dependencies]
ygopro-cdb-encode-rs = { path = "../ygopro-cdb-encode-rs" }
serde_json = "1"
```

## 数据模型

读取 `.cdb` 后，对外暴露的核心类型是 [`CardDataEntry`](./src/model.rs)。

- `datas.id` -> `code`
- `datas.alias` -> `alias` / `rule_code`
- `datas.setcode` -> `setcode: Vec<u16>`
- `datas.type` -> `type_`
- `datas.atk` -> `attack`
- `datas.def` -> `defense` 或 `link_marker`
- `datas.level` -> `level` / `lscale` / `rscale`
- `texts.name` -> `name`
- `texts.desc` -> `desc`
- `texts.str1..str16` -> `strings`

库在读写时会处理几类上游兼容语义：

- Link 怪兽：数据库里的 `def` 会映射到 `link_marker`，`defense` 读出来为 `0`
- `defense` 虚拟字段：对 Link 怪兽视为 `NULL`
- `linkMarker` 虚拟字段：只对 Link 怪兽有值，其它卡视为 `NULL`
- 异画 / 规则卡：会把数据库里的 alias 规范化为 `alias` 和 `rule_code`
- 特例 `5405695`：按上游规则把 alias 归入 `rule_code`
- Token：不会把 alias 提升为 `rule_code`

## 快速开始

### 读取现有 CDB

```rust
use serde_json::json;
use ygopro_cdb_encode_rs::YgoProCdb;

fn main() -> ygopro_cdb_encode_rs::Result<()> {
    let cdb = YgoProCdb::from_path("cards.cdb")?;

    let blue_eyes = cdb.query_raw_one_with(
        "texts.name = :name",
        vec![("name", json!("青眼白龙"))],
    )?;

    println!("{blue_eyes:#?}");
    Ok(())
}
```

### 创建新库并写入卡片

```rust
use ygopro_cdb_encode_rs::{CardDataEntry, CardDataEntryPartial, YgoProCdb};

fn main() -> ygopro_cdb_encode_rs::Result<()> {
    let mut cdb = YgoProCdb::new()?;

    let card = CardDataEntry::default().from_partial(CardDataEntryPartial {
        code: Some(123456),
        name: Some("测试卡".to_string()),
        desc: Some("测试描述".to_string()),
        attack: Some(1800),
        defense: Some(1000),
        level: Some(4),
        strings: Some(vec!["效果1".to_string(), "效果2".to_string()]),
        ..Default::default()
    });

    cdb.add_card(card)?;
    let bytes = cdb.export()?;

    std::fs::write("out.cdb", bytes)?;
    Ok(())
}
```

### 导出 JSON 检查读取效果

```rust
use ygopro_cdb_encode_rs::YgoProCdb;

fn main() -> ygopro_cdb_encode_rs::Result<()> {
    let cdb = YgoProCdb::from_path("cards.cdb")?;
    let cards = cdb.find_all()?;
    let json = serde_json::to_string_pretty(&cards).unwrap();
    std::fs::write("cards.json", json)?;
    Ok(())
}
```

## 查询 API

### 1. 原始 SQL `WHERE` 查询

`query_raw` 只接收 `WHERE` 后面的部分，不需要写完整 `SELECT`。

```rust
use std::collections::HashMap;
use serde_json::json;
use ygopro_cdb_encode_rs::YgoProCdb;

fn demo(cdb: &YgoProCdb) -> ygopro_cdb_encode_rs::Result<()> {
    let mut params = HashMap::new();
    params.insert("name".to_string(), json!("黑魔术师"));
    params.insert("atk".to_string(), json!(2000));

    let cards = cdb.query_raw(
        "texts.name = :name AND datas.atk >= :atk",
        &params,
    )?;

    println!("{}", cards.len());
    Ok(())
}
```

更常用的是 `query_raw_with`：

```rust
use serde_json::json;

let cards = cdb.query_raw_with(
    "texts.name = :name AND datas.atk >= :atk",
    vec![
        ("name", json!("黑魔术师")),
        ("atk", json!(2000)),
    ],
)?;
```

单条结果查询：

```rust
let card = cdb.query_raw_one_with(
    "datas.id = :id",
    vec![("id", serde_json::json!(46986414))],
)?;
```

### 2. Filter DSL 查询

```rust
use ygopro_cdb_encode_rs::{FindFilter, FilterCondition, more_than_or_equal};

let filter = FindFilter::new()
    .with("name", FilterCondition::eq("黑魔术师"))
    .with("attack", more_than_or_equal(2000));

let cards = cdb.find(&filter)?;
```

### 3. 运算符辅助函数

```rust
use ygopro_cdb_encode_rs::{
    FindFilter, FilterCondition, and, has_all_bits, more_than, not, or,
};

let filter = FindFilter::new()
    .with("type", has_all_bits(0x4000000))
    .with("attack", and([
        more_than(2000),
        not(FilterCondition::eq(9999_u32)),
    ]));

let results = cdb.find(&filter)?;

let name_filter = FindFilter::new().with(
    "name",
    or([
        FilterCondition::eq("青眼白龙"),
        FilterCondition::eq("黑魔术师"),
    ]),
);
```

## 可用于 Filter 的字段

基础字段：

- `id`
- `ot`
- `alias`
- `setcode`
- `type`
- `attack`
- `atk`
- `rawDefense`
- `def`
- `race`
- `attribute`
- `category`
- `name`
- `desc`
- `str1` 到 `str16`

虚拟字段：

- `code`
- `defense`
- `linkMarker`
- `rawLevel`
- `level`
- `lscale`
- `rscale`
- `ruleCode`

## `no_texts` 模式

有些数据库可能没有 `texts` 表，或者你只想处理 `datas`。这时可以启用 `no_texts`：

```rust
let mut cdb = YgoProCdb::from_path("cards.cdb")?;
cdb.no_texts(true)?;

let cards = cdb.query_raw_with(
    "datas.id = :id",
    vec![("id", serde_json::json!(100000001))],
)?;
```

启用后需要注意：

- 查询会只从 `datas` 表读取
- `name`、`desc`、`strings` 会返回空字符串
- 任何依赖 `texts.*` 或文本字段 filter 的查询都会返回错误

## API 概览

`YgoProCdb` 目前最常用的方法：

- `new() -> Result<YgoProCdb>`
- `from_path(path) -> Result<YgoProCdb>`
- `from_bytes(bytes) -> Result<YgoProCdb>`
- `export() -> Result<Vec<u8>>`
- `find_all() -> Result<Vec<CardDataEntry>>`
- `find(&FindFilter) -> Result<Vec<CardDataEntry>>`
- `find_one(&FindFilter) -> Result<Option<CardDataEntry>>`
- `find_by_id(id) -> Result<Option<CardDataEntry>>`
- `query_raw(where_clause, &HashMap<String, serde_json::Value>) -> Result<Vec<CardDataEntry>>`
- `query_raw_with(where_clause, params) -> Result<Vec<CardDataEntry>>`
- `query_raw_one(where_clause, &HashMap<String, serde_json::Value>) -> Result<Option<CardDataEntry>>`
- `query_raw_one_with(where_clause, params) -> Result<Option<CardDataEntry>>`
- `step(&FindFilter) -> Result<IntoIter<CardDataEntry>>`
- `step_raw(where_clause, &HashMap<String, serde_json::Value>) -> Result<IntoIter<CardDataEntry>>`
- `step_raw_with(where_clause, params) -> Result<IntoIter<CardDataEntry>>`
- `add_card(card) -> Result<()>`
- `add_cards(cards) -> Result<()>`
- `update_card(card) -> Result<()>`
- `remove_card(code) -> Result<()>`
- `no_texts(bool) -> Result<&mut YgoProCdb>`

## 测试与示例

运行全部测试：

```bash
cargo test
```

把仓库根目录的 `cards.cdb` 读成 JSON：

```bash
cargo test read_cdb_and_write_json_snapshot -- --nocapture
```

生成文件位于：

- `target/test-artifacts/cards.json`

## 当前对齐状态

当前已经显式测试过的上游兼容行为包括：

- Link 怪兽的 `defense` / `link_marker` 读写
- `NULL` filter 语义
- `HasBit` / `HasAllBits`
- `rule_code`、特例卡、Token、链式异画
- `no_texts` 开关与重建
- 原始 SQL 查询与 DSL 查询
- `.cdb` -> `JSON` 的实际读取效果

仍然建议把这个库视为“正在快速对齐上游中的 Rust 实现”，后续可以继续补更多示例、bench、以及与上游测试一一对应的兼容案例。
