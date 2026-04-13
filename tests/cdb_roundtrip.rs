use ygopro_cdb_encode_rs::{
    CardDataEntry, CardDataEntryPartial, FilterCondition, FilterValue, FindFilter, TYPE_LINK,
    TYPE_MONSTER, TYPE_PENDULUM, and, has_all_bits, more_than, not, or,
};

const TYPE_EFFECT: u32 = 0x20;

fn create_link_card() -> CardDataEntry {
    CardDataEntry {
        code: 100_000_001,
        alias: 0,
        setcode: vec![0x1234, 0x5678],
        type_: TYPE_MONSTER | TYPE_EFFECT | TYPE_LINK,
        attack: 2300,
        defense: 0,
        level: 0,
        race: 0x2000,
        attribute: 0x10,
        category: 0,
        ot: 4,
        name: "Link Test".to_string(),
        desc: "A test link monster.".to_string(),
        strings: vec!["str1".to_string()],
        lscale: 0,
        rscale: 0,
        link_marker: 0b10101010,
        rule_code: 0,
    }
}

fn create_pendulum_card() -> CardDataEntry {
    CardDataEntry {
        code: 100_000_002,
        alias: 100_000_900,
        setcode: vec![0x1357],
        type_: TYPE_MONSTER | TYPE_EFFECT | TYPE_PENDULUM,
        attack: 1800,
        defense: 1200,
        level: 7,
        race: 0x1,
        attribute: 0x20,
        category: 0,
        ot: 4,
        name: "Pendulum Test".to_string(),
        desc: "A test pendulum monster.".to_string(),
        strings: vec!["alpha".to_string(), "beta".to_string()],
        lscale: 8,
        rscale: 1,
        link_marker: 0,
        rule_code: 0,
    }
}

#[test]
fn round_trip_preserves_virtual_fields() {
    let mut cdb = ygopro_cdb_encode_rs::YgoProCdb::new().expect("create cdb");
    cdb.add_cards(&[create_link_card(), create_pendulum_card()])
        .expect("insert cards");

    let exported = cdb.export().expect("export db");
    let reopened = ygopro_cdb_encode_rs::YgoProCdb::from_bytes(exported).expect("reopen db");

    let link = reopened
        .find_by_id(100_000_001)
        .expect("query")
        .expect("card");
    assert_eq!(link.link_marker, 0b10101010);
    assert_eq!(link.defense, 0);
    assert_eq!(link.alias, 0);

    let pendulum = reopened
        .find_by_id(100_000_002)
        .expect("query")
        .expect("card");
    assert_eq!(pendulum.level, 7);
    assert_eq!(pendulum.lscale, 8);
    assert_eq!(pendulum.rscale, 1);
    assert_eq!(pendulum.rule_code, 100_000_900);
    assert_eq!(pendulum.alias, 0);
}

#[test]
fn filter_supports_virtual_fields_and_bit_checks() {
    let mut cdb = ygopro_cdb_encode_rs::YgoProCdb::new().expect("create cdb");
    cdb.add_cards(&[create_link_card(), create_pendulum_card()])
        .expect("insert cards");

    let filter = FindFilter::new()
        .with("type", FilterCondition::has_bit(u64::from(TYPE_LINK)))
        .with("name", FilterCondition::eq("Link Test"));

    let found = cdb.find(&filter).expect("find");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].code, 100_000_001);

    let filter = FindFilter::new().with("ruleCode", FilterCondition::more_than(0_u32));
    let found = cdb.find(&filter).expect("find");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].code, 100_000_002);
}

#[test]
fn step_matches_find_results() {
    let mut cdb = ygopro_cdb_encode_rs::YgoProCdb::new().expect("create cdb");
    cdb.add_cards(&[create_link_card(), create_pendulum_card()])
        .expect("insert cards");

    let filter = FindFilter::new().with("id", FilterCondition::eq(100_000_001_u32));
    let from_step: Vec<_> = cdb.step(&filter).expect("step").collect();
    let from_find = cdb.find(&filter).expect("find");

    assert_eq!(from_step, from_find);
}

#[test]
fn no_texts_mode_rejects_text_filters_and_returns_empty_text_fields() {
    let mut cdb = ygopro_cdb_encode_rs::YgoProCdb::new().expect("create cdb");
    cdb.add_card(create_pendulum_card()).expect("insert card");
    cdb.no_texts(true).expect("enable no_texts");

    let found = cdb.find_by_id(100_000_002).expect("query").expect("card");
    assert_eq!(found.name, "");
    assert_eq!(found.desc, "");
    assert!(found.strings.iter().all(|value| value.is_empty()));

    let err = cdb
        .find(&FindFilter::new().with("name", FilterCondition::eq("Pendulum Test")))
        .expect_err("name filter should be rejected");
    assert!(err.to_string().contains("no_texts"));

    let mut params = std::collections::HashMap::new();
    params.insert("name".to_string(), serde_json::Value::from("Pendulum Test"));
    let err = cdb
        .query_raw("texts.name = :name", &params)
        .expect_err("raw text query should be rejected");
    assert!(err.to_string().contains("no_texts"));
}

#[test]
fn null_filter_values_map_to_sql_null_checks() {
    let mut cdb = ygopro_cdb_encode_rs::YgoProCdb::new().expect("create cdb");
    cdb.add_cards(&[create_link_card(), create_pendulum_card()])
        .expect("insert cards");

    let defense_null = cdb
        .find(&FindFilter::new().with("defense", FilterCondition::eq(FilterValue::Null)))
        .expect("find null defense");
    assert_eq!(defense_null.len(), 1);
    assert_eq!(defense_null[0].code, 100_000_001);

    let marker_not_null = cdb
        .find(&FindFilter::new().with("linkMarker", FilterCondition::NotEq(FilterValue::Null)))
        .expect("find non-null marker");
    assert_eq!(marker_not_null.len(), 1);
    assert_eq!(marker_not_null[0].code, 100_000_001);
}

#[test]
fn chained_alternative_artwork_inherits_rule_code() {
    let mut cdb = ygopro_cdb_encode_rs::YgoProCdb::new().expect("create cdb");
    let original = CardDataEntry {
        code: 100_000_010,
        name: "Original".to_string(),
        type_: TYPE_MONSTER,
        ..Default::default()
    };
    let first_alt = CardDataEntry {
        code: 100_000_100,
        alias: 100_000_010,
        name: "Alt 1".to_string(),
        type_: TYPE_MONSTER,
        ..Default::default()
    };
    let second_alt = CardDataEntry {
        code: 100_000_101,
        alias: 100_000_100,
        name: "Alt 2".to_string(),
        type_: TYPE_MONSTER,
        ..Default::default()
    };

    cdb.add_cards(&[original, first_alt, second_alt])
        .expect("insert cards");

    let found = cdb.find_by_id(100_000_101).expect("query").expect("card");
    assert_eq!(found.alias, 100_000_100);
    assert_eq!(found.rule_code, 100_000_010);
}

#[test]
fn helper_operators_match_upstream_style() {
    let mut cdb = ygopro_cdb_encode_rs::YgoProCdb::new().expect("create cdb");
    cdb.add_cards(&[create_link_card(), create_pendulum_card()])
        .expect("insert cards");

    let filter = FindFilter::new()
        .with("type", has_all_bits(u64::from(TYPE_LINK)))
        .with("attack", and([more_than(2000), not(FilterCondition::eq(9999_u32))]));
    let found = cdb.find(&filter).expect("find");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].code, 100_000_001);

    let filter = FindFilter::new().with(
        "name",
        or([
            FilterCondition::eq("Link Test"),
            FilterCondition::eq("Pendulum Test"),
        ]),
    );
    let found = cdb.find(&filter).expect("find");
    assert_eq!(found.len(), 2);
}

#[test]
fn card_data_entry_from_partial_fills_expected_fields() {
    let card = CardDataEntry::default().from_partial(CardDataEntryPartial {
        code: Some(123_456),
        name: Some("Test".to_string()),
        desc: Some("Desc".to_string()),
        attack: Some(1800),
        defense: Some(1000),
        level: Some(4),
        strings: Some(vec!["s1".to_string(), "s2".to_string()]),
        ..Default::default()
    });

    assert_eq!(card.code, 123_456);
    assert_eq!(card.name, "Test");
    assert_eq!(card.desc, "Desc");
    assert_eq!(card.attack, 1800);
    assert_eq!(card.defense, 1000);
    assert_eq!(card.level, 4);
    assert_eq!(card.strings.len(), 16);
    assert_eq!(card.strings[0], "s1");
    assert_eq!(card.strings[1], "s2");
    assert!(card.strings[2..].iter().all(|value| value.is_empty()));
}

#[test]
fn special_card_5405695_moves_alias_to_rule_code() {
    let mut cdb = ygopro_cdb_encode_rs::YgoProCdb::new().expect("create cdb");
    cdb.add_card(CardDataEntry {
        code: 5_405_695,
        alias: 12_345,
        type_: TYPE_MONSTER,
        name: "Special".to_string(),
        ..Default::default()
    })
    .expect("insert card");

    let found = cdb.find_by_id(5_405_695).expect("query").expect("card");
    assert_eq!(found.alias, 0);
    assert_eq!(found.rule_code, 12_345);
}

#[test]
fn token_cards_do_not_move_alias_to_rule_code() {
    let mut cdb = ygopro_cdb_encode_rs::YgoProCdb::new().expect("create cdb");
    cdb.add_card(CardDataEntry {
        code: 100_000_300,
        alias: 99_998,
        type_: TYPE_MONSTER | ygopro_cdb_encode_rs::TYPE_TOKEN,
        name: "Token".to_string(),
        ..Default::default()
    })
    .expect("insert card");

    let found = cdb.find_by_id(100_000_300).expect("query").expect("card");
    assert_eq!(found.alias, 99_998);
    assert_eq!(found.rule_code, 0);
}

#[test]
fn chained_alternative_artwork_round_trip_preserves_alias_and_rule_code() {
    let mut cdb = ygopro_cdb_encode_rs::YgoProCdb::new().expect("create cdb");
    cdb.add_cards(&[
        CardDataEntry {
            code: 10_000,
            type_: TYPE_MONSTER,
            name: "Original".to_string(),
            ..Default::default()
        },
        CardDataEntry {
            code: 20_000,
            alias: 10_000,
            type_: TYPE_MONSTER,
            name: "Alt 1".to_string(),
            ..Default::default()
        },
        CardDataEntry {
            code: 20_001,
            alias: 20_000,
            type_: TYPE_MONSTER,
            name: "Alt 2".to_string(),
            ..Default::default()
        },
    ])
    .expect("insert cards");

    let exported = cdb.export().expect("export");
    let reopened = ygopro_cdb_encode_rs::YgoProCdb::from_bytes(exported).expect("reopen");

    let found = reopened.find_by_id(20_001).expect("query").expect("card");
    assert_eq!(found.alias, 20_000);
    assert_eq!(found.rule_code, 10_000);
}

#[test]
fn no_texts_toggle_recreates_empty_text_rows() {
    let mut cdb = ygopro_cdb_encode_rs::YgoProCdb::new().expect("create cdb");
    cdb.add_card(CardDataEntry {
        code: 777_777,
        name: "toggle-test".to_string(),
        desc: "toggle-desc".to_string(),
        strings: vec!["toggle-str1".to_string()],
        ..Default::default()
    })
    .expect("insert card");

    cdb.no_texts(true).expect("drop texts");
    cdb.no_texts(false).expect("recreate texts");

    let found = cdb.find_by_id(777_777).expect("query").expect("card");
    assert_eq!(found.name, "");
    assert_eq!(found.desc, "");
    assert!(found.strings.iter().all(|value| value.is_empty()));
}

#[test]
fn raw_query_helpers_accept_tuple_params() {
    let mut cdb = ygopro_cdb_encode_rs::YgoProCdb::new().expect("create cdb");
    cdb.add_cards(&[create_link_card(), create_pendulum_card()])
        .expect("insert cards");

    let found = cdb
        .query_raw_with(
            "texts.name = :name AND datas.atk >= :atk",
            vec![
                ("name", serde_json::Value::from("Link Test")),
                ("atk", serde_json::Value::from(2000)),
            ],
        )
        .expect("query");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].code, 100_000_001);

    let one = cdb
        .query_raw_one_with(
            "datas.id = :id",
            vec![("id", serde_json::Value::from(100_000_002))],
        )
        .expect("query one")
        .expect("card");
    assert_eq!(one.code, 100_000_002);

    let from_step: Vec<_> = cdb
        .step_raw_with(
            "datas.id = :id",
            vec![("id", serde_json::Value::from(100_000_001))],
        )
        .expect("step raw")
        .collect();
    assert_eq!(from_step.len(), 1);
    assert_eq!(from_step[0].code, 100_000_001);
}

#[test]
fn find_all_resolves_rule_codes() {
    let mut cdb = ygopro_cdb_encode_rs::YgoProCdb::new().expect("create cdb");
    cdb.add_cards(&[
        CardDataEntry {
            code: 10_000,
            type_: TYPE_MONSTER,
            name: "Original".to_string(),
            ..Default::default()
        },
        CardDataEntry {
            code: 20_000,
            alias: 10_000,
            type_: TYPE_MONSTER,
            name: "Alt 1".to_string(),
            ..Default::default()
        },
        CardDataEntry {
            code: 20_001,
            alias: 20_000,
            type_: TYPE_MONSTER,
            name: "Alt 2".to_string(),
            ..Default::default()
        },
    ])
    .expect("insert cards");

    let all = cdb.find_all().expect("find_all");
    let alt2 = all.iter().find(|c| c.code == 20_001).expect("find alt2");
    // Before the fix, find_all() did NOT call resolve_rule_codes,
    // so rule_code would have been 0. Now it should be properly resolved.
    assert_eq!(alt2.alias, 20_000);
    assert_eq!(alt2.rule_code, 10_000);
}

#[test]
fn filter_value_serde_preserves_unsigned() {
    // Non-negative integers should deserialize as Unsigned
    let json_42 = serde_json::json!(42);
    let val: FilterValue = serde_json::from_value(json_42).expect("deserialize 42");
    assert_eq!(val, FilterValue::Unsigned(42));

    // Negative integers should deserialize as Integer
    let json_neg = serde_json::json!(-5);
    let val: FilterValue = serde_json::from_value(json_neg).expect("deserialize -5");
    assert_eq!(val, FilterValue::Integer(-5));

    // Large u64 values should be preserved
    let large: u64 = u64::MAX;
    let json_large = serde_json::json!(large);
    let val: FilterValue = serde_json::from_value(json_large).expect("deserialize large u64");
    assert_eq!(val, FilterValue::Unsigned(large));

    // Null should deserialize as Null
    let json_null = serde_json::json!(null);
    let val: FilterValue = serde_json::from_value(json_null).expect("deserialize null");
    assert_eq!(val, FilterValue::Null);

    // String should deserialize as Text
    let json_str = serde_json::json!("hello");
    let val: FilterValue = serde_json::from_value(json_str).expect("deserialize string");
    assert_eq!(val, FilterValue::Text("hello".to_string()));
}
