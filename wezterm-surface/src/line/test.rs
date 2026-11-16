#![cfg(test)]

use super::*;
use crate::hyperlink::{Hyperlink, Rule};
use crate::line::clusterline::ClusteredLine;
use crate::line::linebits::LineBits;
use crate::line::storage::CellStorage;
use crate::SequenceNo;
use crate::SEQ_ZERO;
use alloc::sync::Arc;
use k9::assert_equal as assert_eq;
use wezterm_cell::{Cell, CellAttributes};

/// There are 4 double-wide graphemes that occupy 2 cells each.
/// When we join the lines, we must preserve the invisible blank
/// that is part of the grapheme otherwise our metrics will be
/// wrong.
/// <https://github.com/wezterm/wezterm/issues/2568>
#[test]
fn append_line() {
    let mut line1: Line = "0123456789".into();
    let line2: Line = "グループaa".into();

    line1.append_line(line2, SEQ_ZERO);

    assert_eq!(line1.len(), 20);
}

#[test]
fn hyperlinks() {
    let text = "❤ 😍🤢 http://example.com \u{1f468}\u{1f3fe}\u{200d}\u{1f9b0} http://example.com";

    let rules = vec![
        Rule::new(r"\b\w+://(?:[\w.-]+)\.[a-z]{2,15}\S*\b", "$0").unwrap(),
        Rule::new(r"\b\w+@[\w-]+(\.[\w-]+)+\b", "mailto:$0").unwrap(),
    ];

    let hyperlink = Arc::new(Hyperlink::new_implicit("http://example.com"));
    let hyperlink_attr = CellAttributes::default()
        .set_hyperlink(Some(hyperlink.clone()))
        .clone();

    let mut line: Line = text.into();
    line.scan_and_create_hyperlinks(&rules);
    assert!(line.has_hyperlink());
    assert_eq!(
        line.coerce_vec_storage().to_vec(),
        vec![
            Cell::new_grapheme("❤", CellAttributes::default(), None),
            Cell::new(' ', CellAttributes::default()), // double width spacer
            Cell::new_grapheme("😍", CellAttributes::default(), None),
            Cell::new(' ', CellAttributes::default()), // double width spacer
            Cell::new_grapheme("🤢", CellAttributes::default(), None),
            Cell::new(' ', CellAttributes::default()), // double width spacer
            Cell::new(' ', CellAttributes::default()),
            Cell::new('h', hyperlink_attr.clone()),
            Cell::new('t', hyperlink_attr.clone()),
            Cell::new('t', hyperlink_attr.clone()),
            Cell::new('p', hyperlink_attr.clone()),
            Cell::new(':', hyperlink_attr.clone()),
            Cell::new('/', hyperlink_attr.clone()),
            Cell::new('/', hyperlink_attr.clone()),
            Cell::new('e', hyperlink_attr.clone()),
            Cell::new('x', hyperlink_attr.clone()),
            Cell::new('a', hyperlink_attr.clone()),
            Cell::new('m', hyperlink_attr.clone()),
            Cell::new('p', hyperlink_attr.clone()),
            Cell::new('l', hyperlink_attr.clone()),
            Cell::new('e', hyperlink_attr.clone()),
            Cell::new('.', hyperlink_attr.clone()),
            Cell::new('c', hyperlink_attr.clone()),
            Cell::new('o', hyperlink_attr.clone()),
            Cell::new('m', hyperlink_attr.clone()),
            Cell::new(' ', CellAttributes::default()),
            Cell::new_grapheme(
                // man: dark skin tone, red hair ZWJ emoji grapheme
                "\u{1f468}\u{1f3fe}\u{200d}\u{1f9b0}",
                CellAttributes::default(),
                None,
            ),
            Cell::new(' ', CellAttributes::default()), // double width spacer
            Cell::new(' ', CellAttributes::default()),
            Cell::new('h', hyperlink_attr.clone()),
            Cell::new('t', hyperlink_attr.clone()),
            Cell::new('t', hyperlink_attr.clone()),
            Cell::new('p', hyperlink_attr.clone()),
            Cell::new(':', hyperlink_attr.clone()),
            Cell::new('/', hyperlink_attr.clone()),
            Cell::new('/', hyperlink_attr.clone()),
            Cell::new('e', hyperlink_attr.clone()),
            Cell::new('x', hyperlink_attr.clone()),
            Cell::new('a', hyperlink_attr.clone()),
            Cell::new('m', hyperlink_attr.clone()),
            Cell::new('p', hyperlink_attr.clone()),
            Cell::new('l', hyperlink_attr.clone()),
            Cell::new('e', hyperlink_attr.clone()),
            Cell::new('.', hyperlink_attr.clone()),
            Cell::new('c', hyperlink_attr.clone()),
            Cell::new('o', hyperlink_attr.clone()),
            Cell::new('m', hyperlink_attr.clone()),
        ]
    );
}

#[test]
fn double_click_range_bounds() {
    let line: Line = "hello".into();
    let r = line.compute_double_click_range(200, |_| true);
    assert_eq!(r, DoubleClickRange::Range(200..200));
}

#[test]
fn cluster_representation_basic() {
    let line: Line = "hello".into();
    let mut compressed = line.clone();
    compressed.compress_for_scrollback();
    k9::snapshot!(
        &compressed.cells,
        r#"
C(
    ClusteredLine {
        text: "hello",
        is_double_wide: None,
        clusters: [
            Cluster {
                cell_width: 5,
                attrs: CellAttributes {
                    attributes: 0,
                    intensity: Normal,
                    underline: None,
                    blink: None,
                    italic: false,
                    reverse: false,
                    strikethrough: false,
                    invisible: false,
                    wrapped: false,
                    overline: false,
                    semantic_type: Output,
                    foreground: Default,
                    background: Default,
                    fat: None,
                },
            },
        ],
        len: 5,
        last_cell_width: Some(
            1,
        ),
        last_cell_len: Some(
            1,
        ),
        cell_starts: None,
    },
)
"#
    );
    compressed.coerce_vec_storage();
    assert_eq!(line, compressed);
}

#[test]
fn cluster_representation_double_width() {
    let line: Line = "❤ 😍🤢he❤ 😍🤢llo❤ 😍🤢".into();
    let mut compressed = line.clone();
    compressed.compress_for_scrollback();
    k9::snapshot!(
        &compressed.cells,
        r#"
C(
    ClusteredLine {
        text: "❤ 😍🤢he❤ 😍🤢llo❤ 😍🤢",
        is_double_wide: Some(
            FixedBitSet {
                data: [
                    2626580,
                ],
                length: 23,
            },
        ),
        clusters: [
            Cluster {
                cell_width: 23,
                attrs: CellAttributes {
                    attributes: 0,
                    intensity: Normal,
                    underline: None,
                    blink: None,
                    italic: false,
                    reverse: false,
                    strikethrough: false,
                    invisible: false,
                    wrapped: false,
                    overline: false,
                    semantic_type: Output,
                    foreground: Default,
                    background: Default,
                    fat: None,
                },
            },
        ],
        len: 23,
        last_cell_width: Some(
            2,
        ),
        last_cell_len: Some(
            4,
        ),
        cell_starts: None,
    },
)
"#
    );
    compressed.coerce_vec_storage();
    assert_eq!(line, compressed);
}

#[test]
fn cluster_representation_empty() {
    let line = Line::from_cells(vec![], SEQ_ZERO);

    let mut compressed = line.clone();
    compressed.compress_for_scrollback();
    k9::snapshot!(
        &compressed.cells,
        r#"
C(
    ClusteredLine {
        text: "",
        is_double_wide: None,
        clusters: [],
        len: 0,
        last_cell_width: None,
        last_cell_len: None,
        cell_starts: None,
    },
)
"#
    );
    compressed.coerce_vec_storage();
    assert_eq!(line, compressed);
}

#[test]
fn cluster_wrap_last() {
    let mut line: Line = "hello".into();
    line.compress_for_scrollback();
    line.set_last_cell_was_wrapped(true, 1);
    k9::snapshot!(
        line,
        r#"
Line {
    cells: C(
        ClusteredLine {
            text: "hello",
            is_double_wide: None,
            clusters: [
                Cluster {
                    cell_width: 4,
                    attrs: CellAttributes {
                        attributes: 0,
                        intensity: Normal,
                        underline: None,
                        blink: None,
                        italic: false,
                        reverse: false,
                        strikethrough: false,
                        invisible: false,
                        wrapped: false,
                        overline: false,
                        semantic_type: Output,
                        foreground: Default,
                        background: Default,
                        fat: None,
                    },
                },
                Cluster {
                    cell_width: 1,
                    attrs: CellAttributes {
                        attributes: 2048,
                        intensity: Normal,
                        underline: None,
                        blink: None,
                        italic: false,
                        reverse: false,
                        strikethrough: false,
                        invisible: false,
                        wrapped: true,
                        overline: false,
                        semantic_type: Output,
                        foreground: Default,
                        background: Default,
                        fat: None,
                    },
                },
            ],
            len: 5,
            last_cell_width: Some(
                1,
            ),
            last_cell_len: Some(
                1,
            ),
            cell_starts: None,
        },
    ),
    zones: [],
    seqno: 1,
    bits: LineBits(
        0x0,
    ),
    appdata: Mutex {
        data: None,
        poisoned: false,
        ..
    },
}
"#
    );
}

/// A cell can hold part of what the grapheme rules would otherwise join with
/// the cell that follows it; a terminal produces that by placing each of them
/// in a column of its own. Compressing the line for scrollback has to keep
/// those cells apart, along with their widths, their attributes and the
/// wrapped marker on the last of them.
#[test]
fn cluster_representation_keeps_split_cells() {
    let wrapped = CellAttributes::default().set_wrapped(true).clone();
    let line = Line::from_cells(
        vec![
            Cell::new_grapheme_with_width("\u{1f1ec}", 2, CellAttributes::default()),
            Cell::blank(),
            Cell::new_grapheme_with_width("\u{1f1e7}", 2, wrapped.clone()),
            Cell::blank_with_attrs(wrapped),
        ],
        SEQ_ZERO,
    );

    let mut compressed = line.clone();
    compressed.compress_for_scrollback();

    let cells: Vec<_> = compressed
        .visible_cells()
        .map(|cell| (cell.cell_index(), cell.str().to_string(), cell.width()))
        .collect();
    assert_eq!(
        cells,
        vec![
            (0, "\u{1f1ec}".to_string(), 2),
            (2, "\u{1f1e7}".to_string(), 2),
        ]
    );
    assert_eq!(compressed.len(), 4);

    // The wrapped marker is on the final cell, and can still be cleared and
    // set again once the line is compressed.
    assert!(compressed.last_cell_was_wrapped());
    compressed.set_last_cell_was_wrapped(false, SEQ_ZERO);
    assert!(!compressed.last_cell_was_wrapped());
    compressed.set_last_cell_was_wrapped(true, SEQ_ZERO);
    assert!(compressed.last_cell_was_wrapped());

    compressed.coerce_vec_storage();
    assert_eq!(line, compressed);
}

/// Appending has to keep the cells apart too, otherwise reflow rebuilds a
/// line by joining cells that were deliberately placed in columns of their
/// own.
#[test]
fn cluster_append_keeps_split_cells() {
    let mut line = Line::new(SEQ_ZERO);
    line.set_cell_grapheme(0, "\u{1f1ec}", 2, CellAttributes::default(), SEQ_ZERO);
    line.set_cell_grapheme(2, "\u{1f1e7}", 2, CellAttributes::default(), SEQ_ZERO);

    let cells: Vec<_> = line
        .visible_cells()
        .map(|cell| (cell.cell_index(), cell.str().to_string(), cell.width()))
        .collect();
    assert_eq!(
        cells,
        vec![
            (0, "\u{1f1ec}".to_string(), 2),
            (2, "\u{1f1e7}".to_string(), 2),
        ]
    );
    assert_eq!(line.len(), 4);
}

/// The same for joining two lines back together, where the cells come from
/// a line that recorded its own boundaries.
#[test]
fn cluster_append_line_keeps_split_cells() {
    let mut compressed = Line::from_cells(
        vec![
            Cell::new_grapheme_with_width("\u{1f1e7}", 2, CellAttributes::default()),
            Cell::blank(),
            Cell::new_grapheme_with_width("\u{1f1e8}", 2, CellAttributes::default()),
            Cell::blank(),
        ],
        SEQ_ZERO,
    );
    compressed.compress_for_scrollback();
    assert_eq!(compressed.visible_cells().count(), 2);

    let mut line = Line::new(SEQ_ZERO);
    line.set_cell_grapheme(0, "\u{1f1ec}", 2, CellAttributes::default(), SEQ_ZERO);
    line.append_line(compressed, SEQ_ZERO);

    // One more cell after the offsets are already recorded.
    line.set_cell_grapheme(6, "\u{1f1e9}", 2, CellAttributes::default(), SEQ_ZERO);

    let cells: Vec<_> = line
        .visible_cells()
        .map(|cell| (cell.str().to_string(), cell.width()))
        .collect();
    assert_eq!(
        cells,
        vec![
            ("\u{1f1ec}".to_string(), 2),
            ("\u{1f1e7}".to_string(), 2),
            ("\u{1f1e8}".to_string(), 2),
            ("\u{1f1e9}".to_string(), 2),
        ]
    );
    assert_eq!(line.len(), 8);
}

/// A cell can also hold part of an emoji sequence that the grapheme rules
/// would join through a zero width joiner.
#[test]
fn cluster_append_keeps_joined_emoji_apart() {
    let mut line = Line::new(SEQ_ZERO);
    line.set_cell_grapheme(
        0,
        "\u{1f469}\u{200d}",
        2,
        CellAttributes::default(),
        SEQ_ZERO,
    );
    line.set_cell_grapheme(2, "\u{1f4bb}", 2, CellAttributes::default(), SEQ_ZERO);

    let cells: Vec<_> = line
        .visible_cells()
        .map(|cell| (cell.str().to_string(), cell.width()))
        .collect();
    assert_eq!(
        cells,
        vec![
            ("\u{1f469}\u{200d}".to_string(), 2),
            ("\u{1f4bb}".to_string(), 2),
        ]
    );
    assert_eq!(line.len(), 4);
}

/// Pruning has to take the recorded offsets with it, so that what is left
/// still reads back as the cells it was given.
#[test]
fn cluster_prune_trailing_blanks_with_recorded_cells() {
    let mut line = Line::from_cells(
        vec![
            Cell::new_grapheme_with_width("\u{1f1ec}", 2, CellAttributes::default()),
            Cell::blank(),
            Cell::new_grapheme_with_width("\u{1f1e7}", 2, CellAttributes::default()),
            Cell::blank(),
            Cell::blank(),
            Cell::blank(),
        ],
        SEQ_ZERO,
    );
    line.compress_for_scrollback();
    line.prune_trailing_blanks(SEQ_ZERO);

    let cells: Vec<_> = line
        .visible_cells()
        .map(|cell| (cell.str().to_string(), cell.width()))
        .collect();
    assert_eq!(
        cells,
        vec![("\u{1f1ec}".to_string(), 2), ("\u{1f1e7}".to_string(), 2),]
    );
    assert_eq!(line.len(), 4);

    // Appending after a prune still keeps the cells apart.
    line.set_cell_grapheme(4, "\u{1f1e8}", 2, CellAttributes::default(), SEQ_ZERO);
    let texts: Vec<_> = line
        .visible_cells()
        .map(|cell| cell.str().to_string())
        .collect();
    assert_eq!(
        texts,
        vec![
            "\u{1f1ec}".to_string(),
            "\u{1f1e7}".to_string(),
            "\u{1f1e8}".to_string(),
        ]
    );
}

/// A cell the compact storage cannot record the width of keeps cell storage
/// whichever way it arrives.
#[test]
fn cluster_append_declines_unsupported_widths() {
    let mut line = Line::new(SEQ_ZERO);
    line.set_cell_grapheme(0, "x", 3, CellAttributes::default(), SEQ_ZERO);
    assert_eq!(line.len(), 3);
    let cells: Vec<_> = line
        .visible_cells()
        .map(|cell| (cell.str().to_string(), cell.width()))
        .collect();
    assert_eq!(cells, vec![("x".to_string(), 3)]);

    // Joining such a line onto one that is in compact storage moves the
    // destination out of it first.
    let wide = Line::from_cells(
        vec![
            Cell::new_grapheme_with_width("x", 3, CellAttributes::default()),
            Cell::blank(),
            Cell::blank(),
        ],
        SEQ_ZERO,
    );
    let mut destination = Line::new(SEQ_ZERO);
    destination.set_cell_grapheme(0, "a", 1, CellAttributes::default(), SEQ_ZERO);
    assert!(matches!(&destination.cells, CellStorage::C(_)));
    destination.append_line(wide, SEQ_ZERO);
    assert!(matches!(&destination.cells, CellStorage::V(_)));
    assert_eq!(destination.len(), 4);
}

/// The wrapped marker belongs to the cell that ends the row, and a cell the
/// compact storage cannot record still has to carry it.
#[test]
fn wrap_marker_on_unsupported_width() {
    let line = Line::from_cells(
        vec![
            Cell::new_grapheme_with_width("x", 3, CellAttributes::default()),
            Cell::blank(),
            Cell::blank(),
            Cell::new_grapheme_with_width("y", 1, CellAttributes::default()),
        ],
        SEQ_ZERO,
    );

    let mut rows = line.wrap(3, SEQ_ZERO);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].len(), 3);
    assert_eq!(rows[1].len(), 1);
    assert!(rows[0].last_cell_was_wrapped());

    // Every column the cell covers carries the marker, which is what the
    // compact storage rebuilds them as.
    let marked: Vec<bool> = rows[0]
        .cells_mut()
        .iter()
        .map(|cell| cell.attrs().wrapped())
        .collect();
    assert_eq!(marked, vec![true, true, true]);

    rows[0].set_last_cell_was_wrapped(false, SEQ_ZERO);
    assert!(!rows[0].last_cell_was_wrapped());
    let marked: Vec<bool> = rows[0]
        .cells_mut()
        .iter()
        .map(|cell| cell.attrs().wrapped())
        .collect();
    assert_eq!(marked, vec![false, false, false]);
}

/// A wrapped row whose final cell is wide still compresses, because the
/// marker now reads back the same from either storage.
#[test]
fn wrap_marker_on_wide_cell_compresses() {
    let mut line = Line::from_text("a\u{754c}", &CellAttributes::default(), SEQ_ZERO, None);
    line.set_last_cell_was_wrapped(true, SEQ_ZERO);

    let before = line.clone();
    line.compress_for_scrollback();
    assert!(matches!(&line.cells, CellStorage::C(_)));
    assert!(line.last_cell_was_wrapped());

    line.coerce_vec_storage();
    assert_eq!(line, before);
}

/// Attribute runs are measured in columns and cannot record more than
/// `u16::MAX` of them, so a longer run of matching cells is split.
#[test]
fn cluster_representation_long_run() {
    let width = u16::MAX as usize + 16;
    let line = Line::from_text(
        &"x".repeat(width),
        &CellAttributes::default(),
        SEQ_ZERO,
        None,
    );

    let mut compressed = line.clone();
    compressed.compress_for_scrollback();
    assert_eq!(compressed.len(), width);
    assert_eq!(compressed.visible_cells().count(), width);

    compressed.coerce_vec_storage();
    assert_eq!(line, compressed);
}

/// Pruning works on whole cells. A cell that ends with a space but holds
/// more than one is not a blank and has to survive.
#[test]
fn cluster_prune_trailing_blanks_by_cell() {
    let mut line = Line::from_cells(
        vec![
            Cell::new_grapheme_with_width("a", 1, CellAttributes::default()),
            // U+0600 begins a cluster and takes the space with it.
            Cell::new_grapheme_with_width("\u{600} ", 1, CellAttributes::default()),
            Cell::blank(),
            Cell::blank(),
        ],
        SEQ_ZERO,
    );
    line.compress_for_scrollback();
    line.prune_trailing_blanks(SEQ_ZERO);

    let texts: Vec<_> = line
        .visible_cells()
        .map(|cell| cell.str().to_string())
        .collect();
    assert_eq!(texts, vec!["a".to_string(), "\u{600} ".to_string()]);
    assert_eq!(line.len(), 2);
}

/// A cell wider than two columns, and a covered column that carries
/// attributes of its own, are both outside what the compact storage can
/// hold, so the line stays as it is.
#[test]
fn cluster_representation_declines_what_it_cannot_hold() {
    let mut wide = Line::from_cells(
        vec![
            Cell::new_grapheme_with_width("\u{1f1ec}", 3, CellAttributes::default()),
            Cell::blank(),
            Cell::blank(),
        ],
        SEQ_ZERO,
    );
    let before = wide.clone();
    wide.compress_for_scrollback();
    assert_eq!(wide, before);

    let mut covered =
        Line::from_text_with_wrapped_last_col("\u{754c}", &CellAttributes::default(), SEQ_ZERO);
    let before = covered.clone();
    covered.compress_for_scrollback();
    assert_eq!(covered, before);

    // A wide cell whose covered column was cut away is not there to rebuild.
    let mut truncated = Line::from_text("\u{754c}", &CellAttributes::default(), SEQ_ZERO, None);
    truncated.resize(1, SEQ_ZERO);
    let before = truncated.clone();
    truncated.compress_for_scrollback();
    assert_eq!(truncated, before);
    assert_eq!(truncated.len(), 1);
}

fn bold() -> CellAttributes {
    use wezterm_cell::Intensity;
    let mut attr = CellAttributes::default();
    attr.set_intensity(Intensity::Bold);
    attr
}

#[test]
fn cluster_representation_attributes() {
    let line = Line::from_cells(
        vec![
            Cell::new_grapheme("a", CellAttributes::default(), None),
            Cell::new_grapheme("b", bold(), None),
            Cell::new_grapheme("c", CellAttributes::default(), None),
            Cell::new_grapheme("d", bold(), None),
        ],
        SEQ_ZERO,
    );

    let mut compressed = line.clone();
    compressed.compress_for_scrollback();
    k9::snapshot!(
        &compressed.cells,
        r#"
C(
    ClusteredLine {
        text: "abcd",
        is_double_wide: None,
        clusters: [
            Cluster {
                cell_width: 1,
                attrs: CellAttributes {
                    attributes: 0,
                    intensity: Normal,
                    underline: None,
                    blink: None,
                    italic: false,
                    reverse: false,
                    strikethrough: false,
                    invisible: false,
                    wrapped: false,
                    overline: false,
                    semantic_type: Output,
                    foreground: Default,
                    background: Default,
                    fat: None,
                },
            },
            Cluster {
                cell_width: 1,
                attrs: CellAttributes {
                    attributes: 1,
                    intensity: Bold,
                    underline: None,
                    blink: None,
                    italic: false,
                    reverse: false,
                    strikethrough: false,
                    invisible: false,
                    wrapped: false,
                    overline: false,
                    semantic_type: Output,
                    foreground: Default,
                    background: Default,
                    fat: None,
                },
            },
            Cluster {
                cell_width: 1,
                attrs: CellAttributes {
                    attributes: 0,
                    intensity: Normal,
                    underline: None,
                    blink: None,
                    italic: false,
                    reverse: false,
                    strikethrough: false,
                    invisible: false,
                    wrapped: false,
                    overline: false,
                    semantic_type: Output,
                    foreground: Default,
                    background: Default,
                    fat: None,
                },
            },
            Cluster {
                cell_width: 1,
                attrs: CellAttributes {
                    attributes: 1,
                    intensity: Bold,
                    underline: None,
                    blink: None,
                    italic: false,
                    reverse: false,
                    strikethrough: false,
                    invisible: false,
                    wrapped: false,
                    overline: false,
                    semantic_type: Output,
                    foreground: Default,
                    background: Default,
                    fat: None,
                },
            },
        ],
        len: 4,
        last_cell_width: Some(
            1,
        ),
        last_cell_len: Some(
            1,
        ),
        cell_starts: None,
    },
)
"#
    );
    compressed.coerce_vec_storage();
    assert_eq!(line, compressed);
}

#[test]
fn cluster_append() {
    let mut cl = ClusteredLine::new();
    cl.append(Cell::new_grapheme("h", CellAttributes::default(), None));
    cl.append(Cell::new_grapheme("e", CellAttributes::default(), None));
    cl.append(Cell::new_grapheme("l", bold(), None));
    cl.append(Cell::new_grapheme("l", CellAttributes::default(), None));
    cl.append(Cell::new_grapheme("o", CellAttributes::default(), None));
    k9::snapshot!(
        cl,
        r#"
ClusteredLine {
    text: "hello",
    is_double_wide: None,
    clusters: [
        Cluster {
            cell_width: 2,
            attrs: CellAttributes {
                attributes: 0,
                intensity: Normal,
                underline: None,
                blink: None,
                italic: false,
                reverse: false,
                strikethrough: false,
                invisible: false,
                wrapped: false,
                overline: false,
                semantic_type: Output,
                foreground: Default,
                background: Default,
                fat: None,
            },
        },
        Cluster {
            cell_width: 1,
            attrs: CellAttributes {
                attributes: 1,
                intensity: Bold,
                underline: None,
                blink: None,
                italic: false,
                reverse: false,
                strikethrough: false,
                invisible: false,
                wrapped: false,
                overline: false,
                semantic_type: Output,
                foreground: Default,
                background: Default,
                fat: None,
            },
        },
        Cluster {
            cell_width: 2,
            attrs: CellAttributes {
                attributes: 0,
                intensity: Normal,
                underline: None,
                blink: None,
                italic: false,
                reverse: false,
                strikethrough: false,
                invisible: false,
                wrapped: false,
                overline: false,
                semantic_type: Output,
                foreground: Default,
                background: Default,
                fat: None,
            },
        },
    ],
    len: 5,
    last_cell_width: Some(
        1,
    ),
    last_cell_len: Some(
        1,
    ),
    cell_starts: None,
}
"#
    );
}

#[test]
fn cluster_line_new() {
    let mut line = Line::new(1);
    line.set_cell(
        0,
        Cell::new_grapheme("h", CellAttributes::default(), None),
        1,
    );
    line.set_cell(
        1,
        Cell::new_grapheme("e", CellAttributes::default(), None),
        2,
    );
    line.set_cell(2, Cell::new_grapheme("l", bold(), None), 3);
    line.set_cell(
        3,
        Cell::new_grapheme("l", CellAttributes::default(), None),
        4,
    );
    line.set_cell(
        4,
        Cell::new_grapheme("o", CellAttributes::default(), None),
        5,
    );
    k9::snapshot!(
        line,
        r#"
Line {
    cells: C(
        ClusteredLine {
            text: "hello",
            is_double_wide: None,
            clusters: [
                Cluster {
                    cell_width: 2,
                    attrs: CellAttributes {
                        attributes: 0,
                        intensity: Normal,
                        underline: None,
                        blink: None,
                        italic: false,
                        reverse: false,
                        strikethrough: false,
                        invisible: false,
                        wrapped: false,
                        overline: false,
                        semantic_type: Output,
                        foreground: Default,
                        background: Default,
                        fat: None,
                    },
                },
                Cluster {
                    cell_width: 1,
                    attrs: CellAttributes {
                        attributes: 1,
                        intensity: Bold,
                        underline: None,
                        blink: None,
                        italic: false,
                        reverse: false,
                        strikethrough: false,
                        invisible: false,
                        wrapped: false,
                        overline: false,
                        semantic_type: Output,
                        foreground: Default,
                        background: Default,
                        fat: None,
                    },
                },
                Cluster {
                    cell_width: 2,
                    attrs: CellAttributes {
                        attributes: 0,
                        intensity: Normal,
                        underline: None,
                        blink: None,
                        italic: false,
                        reverse: false,
                        strikethrough: false,
                        invisible: false,
                        wrapped: false,
                        overline: false,
                        semantic_type: Output,
                        foreground: Default,
                        background: Default,
                        fat: None,
                    },
                },
            ],
            len: 5,
            last_cell_width: Some(
                1,
            ),
            last_cell_len: Some(
                1,
            ),
            cell_starts: None,
        },
    ),
    zones: [],
    seqno: 5,
    bits: LineBits(
        0x0,
    ),
    appdata: Mutex {
        data: None,
        poisoned: false,
        ..
    },
}
"#
    );
}

fn text_line(text: &str, seqno: SequenceNo) -> Line {
    Line::from_text(text, &CellAttributes::default(), seqno, None)
}

fn marked(mut line: Line) -> Line {
    line.set_caller_mark(true);
    line
}

#[test]
fn caller_mark_does_not_widen_the_line() {
    assert_eq!(core::mem::size_of::<LineBits>(), 2);
}

#[test]
fn caller_mark_is_absent_on_a_new_line() {
    assert!(!Line::new(1).caller_mark());
    assert!(!Line::with_width(10, 1).caller_mark());
    assert!(!Line::with_width_and_cell(10, Cell::blank(), 1).caller_mark());
    assert!(!Line::from_cells(vec![Cell::blank(); 3], 1).caller_mark());
    assert!(!text_line("abc", 1).caller_mark());
    assert!(!Line::from("abc").caller_mark());
}

#[test]
fn caller_mark_changes_no_cell_and_no_sequence_number() {
    let mut line = text_line("abc def", 7);
    let before_cells: Vec<Cell> = line.visible_cells().map(|c| c.as_cell()).collect();
    let before_seqno = line.current_seqno();

    line.set_caller_mark(true);
    assert!(line.caller_mark());
    assert_eq!(line.current_seqno(), before_seqno);
    assert!(!line.changed_since(before_seqno));
    let after_cells: Vec<Cell> = line.visible_cells().map(|c| c.as_cell()).collect();
    assert_eq!(after_cells, before_cells);
    assert_eq!(line.as_str(), "abc def");

    line.set_caller_mark(false);
    assert!(!line.caller_mark());
    assert_eq!(line.current_seqno(), before_seqno);
    assert_eq!(line, text_line("abc def", 7));
}

#[test]
fn caller_mark_leaves_the_other_bits_alone() {
    let mut line = text_line("abc", 1);
    line.set_double_width(2);
    line.set_caller_mark(true);
    assert!(line.is_double_width());
    line.set_caller_mark(false);
    assert!(line.is_double_width());
    line.set_caller_mark(true);
    line.set_single_width(3);
    assert!(line.caller_mark());
}

#[test]
fn caller_mark_is_not_part_of_the_shape_hash() {
    let plain = text_line("abc", 1);
    let flagged = marked(text_line("abc", 1));
    assert_eq!(plain.compute_shape_hash(), flagged.compute_shape_hash());
}

#[test]
fn caller_mark_follows_a_clone() {
    let line = marked(text_line("abc", 1));
    assert!(line.clone().caller_mark());
    assert!(!text_line("abc", 1).clone().caller_mark());
}

#[test]
fn caller_mark_follows_both_halves_of_a_split() {
    let mut head = marked(text_line("abcdef", 1));
    let tail = head.split_off(3, 2);
    assert!(head.caller_mark());
    assert!(tail.caller_mark());
    assert_eq!(tail.as_str(), "def");

    let mut head = text_line("abcdef", 1);
    let tail = head.split_off(3, 2);
    assert!(!head.caller_mark());
    assert!(!tail.caller_mark());
}

#[test]
fn caller_mark_follows_every_line_that_a_wrap_makes() {
    let lines = marked(text_line("abcdefghij", 1)).wrap(4, 2);
    assert_eq!(lines.len(), 3);
    for line in &lines {
        assert!(line.caller_mark());
    }
    let texts: Vec<String> = lines
        .iter()
        .map(|l| l.as_str().trim_end().to_string())
        .collect();
    assert_eq!(
        texts,
        vec!["abcd".to_string(), "efgh".to_string(), "ij".to_string()]
    );

    let lines = text_line("abcdefghij", 1).wrap(4, 2);
    assert_eq!(lines.len(), 3);
    for line in &lines {
        assert!(!line.caller_mark());
    }
}

#[test]
fn caller_mark_follows_a_wrap_that_makes_one_line() {
    let lines = marked(text_line("abc", 1)).wrap(10, 2);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].caller_mark());
}

#[test]
fn caller_mark_follows_a_wrap_of_a_blank_line() {
    let lines = marked(Line::with_width(10, 1)).wrap(4, 2);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].caller_mark());
}

#[test]
fn caller_mark_follows_the_line_that_is_appended() {
    let mut line = text_line("0123", 1);
    line.append_line(marked(text_line("4567", 1)), 2);
    assert!(line.caller_mark());
    assert_eq!(line.as_str(), "01234567");

    let mut line = marked(text_line("0123", 1));
    line.append_line(text_line("4567", 1), 2);
    assert!(line.caller_mark());

    let mut line = text_line("0123", 1);
    line.append_line(text_line("4567", 1), 2);
    assert!(!line.caller_mark());
}

#[test]
fn caller_mark_follows_an_append_into_a_wide_cell_line() {
    let mut line = text_line("グループ", 1);
    line.append_line(marked(text_line("aa", 1)), 2);
    assert!(line.caller_mark());
}

#[test]
fn caller_mark_is_removed_by_clearing_the_line() {
    let mut line = marked(text_line("abc", 1));
    line.resize_and_clear(5, 2, CellAttributes::default());
    assert!(!line.caller_mark());
}
