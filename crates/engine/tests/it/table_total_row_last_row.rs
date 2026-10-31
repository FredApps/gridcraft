use gridcraft_core::{CellRef, MAX_ROWS};
use gridcraft_engine::Session;
use serde_json::json;

fn refused(range: &str) {
    let mut s = Session::new();
    s.new_workbook();
    s.execute("insert.table", json!({"range": range, "header": true})).unwrap();
    let before = s.doc().unwrap().wb.active().unwrap().tables[0].clone();
    assert!(s.execute("table.totalRow", json!({"on": true, "table": &before.name})).is_err(), "{range}");
    let sh = s.doc().unwrap().wb.active().unwrap();
    assert_eq!(sh.tables[0].range, before.range, "{range}");
    assert_eq!(sh.tables[0].range.end.row, MAX_ROWS - 1, "{range}");
    assert!(!sh.tables[0].totals_row, "{range}");
    assert!(sh.value(CellRef::new(MAX_ROWS, 0)).is_empty(), "{range}");
}

#[test]
fn total_row_is_refused_on_a_table_ending_at_the_last_worksheet_row() {
    refused("A1048575:B1048576");
    refused("A1048576:B1048576");
}

#[test]
fn total_row_still_grows_an_ordinary_table() {
    let mut s = Session::new();
    s.new_workbook();
    s.execute("range.setValues", json!({"range": "A1", "values": [["Item", "Amount"], ["a", 2], ["b", 3]]})).unwrap();
    s.execute("insert.table", json!({"range": "A1:B3", "header": true})).unwrap();
    s.execute("table.totalRow", json!({"on": true, "table": "Table1"})).unwrap();
    let t = &s.doc().unwrap().wb.active().unwrap().tables[0];
    assert!(t.totals_row);
    assert_eq!(t.range.end.row, 3);
}

#[test]
fn total_row_cannot_overlap_a_merge_or_another_table() {
    for merged in [true, false] {
        let mut s = Session::new();
        s.new_workbook();
        s.execute("insert.table", json!({"range": "B2:D4", "header": true})).unwrap();
        if merged {
            s.execute("home.mergeCenter", json!({"range": "C5:E7"})).unwrap();
        } else {
            s.execute("insert.table", json!({"range": "C5:E7", "header": true})).unwrap();
        }
        let before = s.doc().unwrap().wb.clone();
        let undo = s.doc().unwrap().undo.len();
        assert!(s.execute("table.totalRow", json!({"on": true, "table": "Table1"})).is_err());
        assert_eq!(s.doc().unwrap().wb.active().unwrap().tables[0].range, before.active().unwrap().tables[0].range);
        assert!(!s.doc().unwrap().wb.active().unwrap().tables[0].totals_row);
        assert_eq!(s.doc().unwrap().undo.len(), undo);
    }
}
