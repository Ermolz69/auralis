#![allow(clippy::expect_used)]

use glib::variant::ToVariant;

#[test]
fn next_reads_the_ffi_out_pointer() {
    let value = ["first", "second"].to_variant();
    let mut iterator = value.array_iter_str().expect("string array");
    assert_eq!(iterator.next(), Some("first"));
    assert_eq!(iterator.next(), Some("second"));
    assert_eq!(iterator.next(), None);
}

#[test]
fn reverse_and_skipping_preserve_borrowed_utf8_values() {
    let values = ["", "αβ", "字幕", "🙂", "last"];
    let value = values.to_variant();
    assert_eq!(
        value
            .array_iter_str()
            .expect("string array")
            .collect::<Vec<_>>(),
        values
    );
    assert_eq!(
        value
            .array_iter_str()
            .expect("string array")
            .rev()
            .collect::<Vec<_>>(),
        values.into_iter().rev().collect::<Vec<_>>()
    );
    assert_eq!(
        value.array_iter_str().expect("string array").nth(2),
        Some("字幕")
    );
    assert_eq!(
        value.array_iter_str().expect("string array").nth_back(1),
        Some("🙂")
    );
    assert_eq!(
        value.array_iter_str().expect("string array").last(),
        Some("last")
    );
}

#[test]
fn mixed_directions_stop_at_the_same_boundary() {
    let value = ["a", "b", "c", "d", "e"].to_variant();
    let mut iterator = value.array_iter_str().expect("string array");
    assert_eq!(iterator.len(), 5);
    assert_eq!(iterator.next(), Some("a"));
    assert_eq!(iterator.next_back(), Some("e"));
    assert_eq!(iterator.nth(1), Some("c"));
    assert_eq!(iterator.nth_back(0), Some("d"));
    assert_eq!(iterator.len(), 0);
    assert_eq!(iterator.next(), None);
    assert_eq!(iterator.next_back(), None);
}

#[test]
fn empty_and_out_of_bounds_iterations_are_safe() {
    let value = Vec::<String>::new().to_variant();
    let mut iterator = value.array_iter_str().expect("string array");
    assert_eq!(iterator.next(), None);
    assert_eq!(iterator.next_back(), None);
    let value = ["only"].to_variant();
    assert_eq!(
        value
            .array_iter_str()
            .expect("string array")
            .nth(usize::MAX),
        None
    );
    assert_eq!(
        value
            .array_iter_str()
            .expect("string array")
            .nth_back(usize::MAX),
        None
    );
}
