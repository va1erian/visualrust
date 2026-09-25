//! Unit tests for the pure string API.
//!
//! These call the Rust functions directly, so a failure points at the string
//! logic rather than at Dyon's marshalling.

use super::*;

/// Unwraps the error of a failing call for comparison.
fn failure<T>(result: Result<T, StringError>) -> StringError {
    match result {
        Ok(_) => panic!("expected a StringError"),
        Err(error) => error,
    }
}

#[test]
fn left_takes_a_prefix_and_clamps() {
    assert_eq!(left("VisualRust", 6).unwrap(), "Visual");
    assert_eq!(left("abc", 0).unwrap(), "");
    assert_eq!(left("abc", 99).unwrap(), "abc");
    assert_eq!(left("", 5).unwrap(), "");
}

#[test]
fn left_rejects_negative_lengths() {
    assert_eq!(
        failure(left("abc", -1)),
        StringError::NegativeLength {
            function: "left",
            value: -1
        }
    );
}

#[test]
fn right_takes_a_suffix_and_clamps() {
    assert_eq!(right("VisualRust", 4).unwrap(), "Rust");
    assert_eq!(right("abc", 0).unwrap(), "");
    assert_eq!(right("abc", 99).unwrap(), "abc");
    assert_eq!(right("", 5).unwrap(), "");
}

#[test]
fn right_rejects_negative_lengths() {
    assert_eq!(
        failure(right("abc", -2)),
        StringError::NegativeLength {
            function: "right",
            value: -2
        }
    );
}

#[test]
fn mid_uses_a_one_based_start() {
    assert_eq!(mid("VisualRust", 7, 4).unwrap(), "Rust");
    assert_eq!(mid("abc", 2, 99).unwrap(), "bc");
    assert_eq!(mid("abc", 1, 0).unwrap(), "");
}

#[test]
fn mid_rejects_out_of_range_and_negative_lengths() {
    assert_eq!(
        failure(mid("abc", 0, 1)),
        StringError::IndexOutOfRange {
            function: "mid",
            index: 0,
            count: 3
        }
    );
    assert_eq!(
        failure(mid("abc", 4, 1)),
        StringError::IndexOutOfRange {
            function: "mid",
            index: 4,
            count: 3
        }
    );
    assert_eq!(
        failure(mid("", 1, 1)),
        StringError::IndexOutOfRange {
            function: "mid",
            index: 1,
            count: 0
        }
    );
    assert_eq!(
        failure(mid("abc", 1, -2)),
        StringError::NegativeLength {
            function: "mid",
            value: -2
        }
    );
}

#[test]
fn len_counts_characters_not_bytes() {
    assert_eq!(len("héllo"), 5);
    assert_eq!(len(""), 0);
    assert_eq!(len("日本語"), 3);
}

#[test]
fn case_mapping_is_unicode_aware() {
    assert_eq!(ucase("dyn"), "DYN");
    assert_eq!(lcase("DYn"), "dyn");
    assert_eq!(ucase("ß"), "SS");
    assert_eq!(ucase(""), "");
}

#[test]
fn find_string_is_one_based_and_zero_when_absent() {
    assert_eq!(find_string("VisualRust", "Rust"), 7);
    assert_eq!(find_string("héllo", "l"), 3);
    assert_eq!(find_string("abc", "z"), 0);
    assert_eq!(find_string("", "x"), 0);
    assert_eq!(find_string("abc", ""), 1);
}

#[test]
fn replace_string_replaces_all_non_overlapping_matches() {
    assert_eq!(replace_string("a-b-c", "-", "+"), "a+b+c");
    assert_eq!(replace_string("aaaa", "aa", "b"), "bb");
    assert_eq!(replace_string("naïve", "ï", "i"), "naive");
    assert_eq!(replace_string("abc", "z", "x"), "abc");
    assert_eq!(replace_string("abc", "", "x"), "abc");
}

#[test]
fn trims_remove_unicode_whitespace() {
    assert_eq!(trim(" \t hi \n"), "hi");
    assert_eq!(ltrim("  hi  "), "hi  ");
    assert_eq!(rtrim("  hi  "), "  hi");
    assert_eq!(trim("   "), "");
}

#[test]
fn count_string_counts_non_overlapping_matches() {
    assert_eq!(count_string("banana", "an"), 2);
    assert_eq!(count_string("aaaa", "aa"), 2);
    assert_eq!(count_string("abc", "z"), 0);
    assert_eq!(count_string("abc", ""), 0);
}

#[test]
fn insert_string_accepts_the_position_after_the_end() {
    assert_eq!(insert_string("ab", 2, "-").unwrap(), "a-b");
    assert_eq!(insert_string("abc", 4, "X").unwrap(), "abcX");
    assert_eq!(insert_string("", 1, "X").unwrap(), "X");
    assert_eq!(insert_string("héllo", 2, "X").unwrap(), "hXéllo");
}

#[test]
fn insert_string_rejects_out_of_range_positions() {
    assert_eq!(
        failure(insert_string("abc", 0, "X")),
        StringError::PositionOutOfRange {
            function: "insert_string",
            position: 0,
            max: 4
        }
    );
    assert_eq!(
        failure(insert_string("abc", 5, "X")),
        StringError::PositionOutOfRange {
            function: "insert_string",
            position: 5,
            max: 4
        }
    );
}

#[test]
fn remove_string_removes_a_one_based_range() {
    assert_eq!(remove_string("a-b-c", 2, 1).unwrap(), "ab-c");
    assert_eq!(remove_string("abc", 1, 99).unwrap(), "");
    assert_eq!(remove_string("abc", 3, 0).unwrap(), "abc");
}

#[test]
fn remove_string_rejects_bad_arguments() {
    assert_eq!(
        failure(remove_string("abc", 0, 1)),
        StringError::IndexOutOfRange {
            function: "remove_string",
            index: 0,
            count: 3
        }
    );
    assert_eq!(
        failure(remove_string("abc", 2, -1)),
        StringError::NegativeLength {
            function: "remove_string",
            value: -1
        }
    );
}

#[test]
fn string_field_splits_on_the_delimiter() {
    assert_eq!(string_field("a,b,c", 2, ",").unwrap(), "b");
    assert_eq!(string_field("a,b,c", 1, ",").unwrap(), "a");
    assert_eq!(string_field("a, b", 2, ", ").unwrap(), "b");
}

#[test]
fn string_field_is_lenient_about_missing_fields() {
    assert_eq!(string_field("a,b", 5, ",").unwrap(), "");
    assert_eq!(string_field("", 1, ",").unwrap(), "");
    assert_eq!(
        failure(string_field("a,b", 0, ",")),
        StringError::InvalidFieldIndex {
            function: "string_field",
            index: 0
        }
    );
}

#[test]
fn string_field_without_a_delimiter_is_one_field() {
    assert_eq!(string_field("a,b", 1, "").unwrap(), "a,b");
    assert_eq!(string_field("a,b", 2, "").unwrap(), "");
}

#[test]
fn space_builds_spaces_and_rejects_negatives() {
    assert_eq!(space(3).unwrap(), "   ");
    assert_eq!(space(0).unwrap(), "");
    assert_eq!(
        failure(space(-1)),
        StringError::NegativeLength {
            function: "space",
            value: -1
        }
    );
}

#[test]
fn val_reads_the_longest_numeric_prefix() {
    assert_eq!(val("42abc"), 42.0);
    assert_eq!(val("-3.5xyz"), -3.5);
    assert_eq!(val("  7 "), 7.0);
    assert_eq!(val("1e3rest"), 1000.0);
    assert_eq!(val(".5"), 0.5);
    assert_eq!(val("12.34.56"), 12.34);
    assert_eq!(val("abc"), 0.0);
    assert_eq!(val(""), 0.0);
}

#[test]
fn number_to_string_keeps_fractional_values() {
    assert_eq!(number_to_string(3.0), "3");
    assert_eq!(number_to_string(-4.0), "-4");
    assert_eq!(number_to_string(3.5), "3.5");
}

#[test]
fn hex_formats_and_pads() {
    assert_eq!(hex(255.0, 2).unwrap(), "FF");
    assert_eq!(hex(255.0, 0).unwrap(), "FF");
    assert_eq!(hex(10.0, 4).unwrap(), "000A");
    assert_eq!(hex(0.0, 1).unwrap(), "0");
}

#[test]
fn hex_rejects_negative_and_non_integer_values() {
    assert_eq!(
        failure(hex(-1.0, 0)),
        StringError::NotNonNegativeInteger {
            function: "hex",
            value: -1.0
        }
    );
    assert_eq!(
        failure(hex(1.5, 0)),
        StringError::NotNonNegativeInteger {
            function: "hex",
            value: 1.5
        }
    );
    assert_eq!(
        failure(hex(1.0, -1)),
        StringError::NegativeLength {
            function: "hex",
            value: -1
        }
    );
}

#[test]
fn bin_formats_and_pads() {
    assert_eq!(bin(5.0, 4).unwrap(), "0101");
    assert_eq!(bin(5.0, 0).unwrap(), "101");
    assert_eq!(
        failure(bin(-1.0, 0)),
        StringError::NotNonNegativeInteger {
            function: "bin",
            value: -1.0
        }
    );
}

#[test]
fn format_uses_a_fixed_precision() {
    assert_eq!(format(3.5, 2).unwrap(), "3.50");
    assert_eq!(format(2.0, 0).unwrap(), "2");
    assert_eq!(
        failure(format(1.0, -1)),
        StringError::NegativeLength {
            function: "format",
            value: -1
        }
    );
    assert_eq!(
        failure(format(1.0, (MAX_PRECISION + 1) as i64)),
        StringError::LengthTooLarge {
            function: "format",
            value: (MAX_PRECISION + 1) as i64,
            limit: MAX_PRECISION
        }
    );
}

#[test]
fn lset_pads_on_the_right_and_truncates() {
    assert_eq!(lset("ab", 4, "0").unwrap(), "ab00");
    assert_eq!(lset("abcdef", 3, "x").unwrap(), "abc");
    assert_eq!(lset("ab", 4, "").unwrap(), "ab  ");
    assert_eq!(lset("héllo", 3, "").unwrap(), "hél");
}

#[test]
fn rset_pads_on_the_left_and_truncates() {
    assert_eq!(rset("ab", 4, "0").unwrap(), "00ab");
    assert_eq!(rset("abcdef", 3, "x").unwrap(), "def");
    assert_eq!(rset("ab", 4, "").unwrap(), "  ab");
    assert_eq!(rset("héllo", 3, "").unwrap(), "llo");
}

#[test]
fn padding_rejects_oversized_widths() {
    assert_eq!(
        failure(lset("a", (MAX_LENGTH + 1) as i64, "")),
        StringError::LengthTooLarge {
            function: "lset",
            value: (MAX_LENGTH + 1) as i64,
            limit: MAX_LENGTH
        }
    );
}

#[test]
fn to_index_accepts_only_finite_integers() {
    assert_eq!(to_index(3.0, "mid").unwrap(), 3);
    assert_eq!(to_index(-2.0, "mid").unwrap(), -2);
    assert_eq!(
        failure(to_index(1.5, "mid")),
        StringError::NotAnInteger {
            function: "mid",
            value: 1.5
        }
    );
    // NaN never compares equal, so match on the variant instead of asserting.
    assert!(matches!(
        to_index(f64::NAN, "mid"),
        Err(StringError::NotAnInteger {
            function: "mid",
            ..
        })
    ));
}
