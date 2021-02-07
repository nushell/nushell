use super::*;
use nu_test_support::value::*;

use indexmap::indexmap;

#[test]
fn forgiving_insertion_test_1() {
    let field_path = column_path("crate.version").as_column_path().unwrap();

    let version = string("nuno");

    let value = UntaggedValue::row(indexmap! {
        "package".into() =>
            row(indexmap! {
                "name".into()    =>     string("nu"),
                "version".into() =>  string("0.20.0")
            })
    });

    assert_eq!(
        *value
            .into_untagged_value()
            .forgiving_insert_data_at_column_path(&field_path.item, version)
            .unwrap()
            .get_data_by_column_path(&field_path, Box::new(error_callback("crate.version")))
            .unwrap(),
        *string("nuno")
    );
}

#[test]
fn forgiving_insertion_test_2() {
    let field_path = column_path("things.0").as_column_path().unwrap();

    let version = string("arepas");

    let value = UntaggedValue::row(indexmap! {
        "pivot_mode".into() => string("never"),
        "things".into() => table(&[string("frijoles de Andrés"), int(1)]),
        "color_config".into() =>
            row(indexmap! {
                "header_align".into()    =>     string("left"),
                "index_color".into() =>  string("cyan_bold")
            })
    });

    assert_eq!(
        *value
            .into_untagged_value()
            .forgiving_insert_data_at_column_path(&field_path.item, version)
            .unwrap()
            .get_data_by_column_path(&field_path, Box::new(error_callback("things.0")))
            .unwrap(),
        *string("arepas")
    );
}

#[test]
fn forgiving_insertion_test_3() {
    let field_path = column_path("color_config.arepa_color")
        .as_column_path()
        .unwrap();
    let pizza_path = column_path("things.0").as_column_path().unwrap();

    let entry = string("amarillo");

    let value = UntaggedValue::row(indexmap! {
        "pivot_mode".into() => string("never"),
        "things".into() => table(&[string("Arepas de Yehuda"), int(1)]),
        "color_config".into() =>
            row(indexmap! {
                "header_align".into()    =>     string("left"),
                "index_color".into() =>  string("cyan_bold")
            })
    });

    assert_eq!(
        *value
            .clone()
            .into_untagged_value()
            .forgiving_insert_data_at_column_path(&field_path, entry.clone())
            .unwrap()
            .get_data_by_column_path(
                &field_path.item,
                Box::new(error_callback("color_config.arepa_color"))
            )
            .unwrap(),
        *string("amarillo")
    );

    assert_eq!(
        *value
            .into_untagged_value()
            .forgiving_insert_data_at_column_path(&field_path.item, entry)
            .unwrap()
            .get_data_by_column_path(&pizza_path, Box::new(error_callback("things.0")))
            .unwrap(),
        *string("Arepas de Yehuda")
    );
}

#[test]
fn get_row_data_by_key() {
    let row = row(indexmap! {
            "lines".to_string() => int(0),
            "words".to_string() => int(7),
    });
    assert_eq!(
        row.get_data_by_key("lines".spanned_unknown()).unwrap(),
        int(0)
    );
    assert!(row.get_data_by_key("chars".spanned_unknown()).is_none());
}

#[test]
fn get_table_data_by_key() {
    let row1 = row(indexmap! {
            "lines".to_string() => int(0),
            "files".to_string() => int(10),
    });

    let row2 = row(indexmap! {
            "files".to_string() => int(1)
    });

    let table_value = table(&[row1, row2]);
    assert_eq!(
        table_value
            .get_data_by_key("files".spanned_unknown())
            .unwrap(),
        table(&[int(10), int(1)])
    );
    assert_eq!(
        table_value
            .get_data_by_key("chars".spanned_unknown())
            .unwrap(),
        table(&[nothing(), nothing()])
    );
}