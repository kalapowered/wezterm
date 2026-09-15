use crate::line::CellRef;
use core::convert::{TryFrom, TryInto};
use core::num::{NonZeroU16, NonZeroU8};
use finl_unicode::grapheme_clusters::Graphemes;
use fixedbitset::FixedBitSet;
#[cfg(feature = "use_serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use wezterm_cell::{Cell, CellAttributes};

extern crate alloc;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq)]
struct Cluster {
    cell_width: u16,
    attrs: CellAttributes,
}

/// Stores line data as a contiguous string and a series of
/// clusters of attribute data describing attributed ranges
/// within the line
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ClusteredLine {
    pub text: String,
    #[cfg_attr(
        feature = "use_serde",
        serde(
            deserialize_with = "deserialize_bitset",
            serialize_with = "serialize_bitset"
        )
    )]
    is_double_wide: Option<Box<FixedBitSet>>,
    clusters: Vec<Cluster>,
    /// Length, measured in cells
    len: u32,
    last_cell_width: Option<NonZeroU8>,
    /// Byte length of the final cell's text, where it is short enough to
    /// record here. Appending consults it to decide, without walking the
    /// line, whether the text it is about to add begins a cluster of its own.
    last_cell_len: Option<NonZeroU16>,
    /// Byte offsets into `text`, one per cell, recorded only when splitting
    /// `text` into grapheme clusters would not give back the cells that this
    /// line was built from. That happens when a cell holds part of what the
    /// grapheme rules would join, which a terminal can produce by moving the
    /// cursor into the middle of a sequence, or by following a width model
    /// that gives a column to a scalar the rules treat as a continuation.
    /// Boxed so that the usual line, where the text is its own record of
    /// where the cells are, pays a single pointer for it rather than the
    /// three words a vector would add to every clustered line.
    #[allow(clippy::box_collection)]
    cell_starts: Option<Box<Vec<usize>>>,
}

#[cfg(feature = "use_serde")]
fn deserialize_bitset<'de, D>(deserializer: D) -> Result<Option<Box<FixedBitSet>>, D::Error>
where
    D: Deserializer<'de>,
{
    let wide_indices = <Vec<usize>>::deserialize(deserializer)?;
    if wide_indices.is_empty() {
        Ok(None)
    } else {
        let max_idx = wide_indices.iter().max().unwrap_or(&1);
        let mut bitset = FixedBitSet::with_capacity(max_idx + 1);
        for idx in wide_indices {
            bitset.set(idx, true);
        }
        Ok(Some(Box::new(bitset)))
    }
}

/// Serialize the bitset as a vector of the indices of just the 1 bits;
/// the thesis is that most of the cells on a given line are single width.
/// That may not be strictly true for users that heavily use asian scripts,
/// but we'll start with this and see if we need to improve it.
#[cfg(feature = "use_serde")]
fn serialize_bitset<S>(value: &Option<Box<FixedBitSet>>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut wide_indices: Vec<usize> = vec![];
    if let Some(bits) = value {
        for idx in bits.ones() {
            wide_indices.push(idx);
        }
    }
    wide_indices.serialize(serializer)
}

/// The byte offsets into `text` at which each grapheme cluster begins.
fn grapheme_starts(text: &str) -> Vec<usize> {
    let mut starts = vec![];
    let mut offset = 0;
    for grapheme in Graphemes::new(text) {
        starts.push(offset);
        offset += grapheme.len();
    }
    starts
}

/// Whether splitting `text` into grapheme clusters starts a cluster at every
/// offset in `starts`, and nowhere else. When it does, the text is its own
/// record of where the cells are and the offsets need not be kept.
fn grapheme_starts_match(text: &str, starts: &[usize]) -> bool {
    let mut offset = 0;
    let mut seen = 0;
    for grapheme in Graphemes::new(text) {
        match starts.get(seen) {
            Some(&start) if start == offset => {}
            _ => return false,
        }
        offset += grapheme.len();
        seen += 1;
    }
    seen == starts.len()
}

impl ClusteredLine {
    pub fn new() -> Self {
        Self {
            text: String::with_capacity(80),
            is_double_wide: None,
            clusters: vec![],
            len: 0,
            last_cell_width: None,
            last_cell_len: None,
            cell_starts: None,
        }
    }

    fn set_last_cell_len(&mut self, len: usize) {
        self.last_cell_len = u16::try_from(len).ok().and_then(NonZeroU16::new);
    }

    /// Appends the text of one cell, and starts recording where the cells
    /// are if the line's text stops being a faithful record of that by
    /// itself.
    fn push_cell_text(&mut self, text: &str) {
        let start = self.text.len();
        if let Some(starts) = self.cell_starts.as_mut() {
            starts.push(start);
            self.text.push_str(text);
            self.set_last_cell_len(text.len());
            return;
        }

        self.text.push_str(text);
        if !self.starts_its_own_cluster(start, text.len()) {
            // The grapheme rules would join this cell to the one before it,
            // or divide it in two, so from here on the line carries the
            // offsets of its cells.
            let mut starts = grapheme_starts(&self.text[..start]);
            starts.push(start);
            self.cell_starts = Some(Box::new(starts));
        }
        self.set_last_cell_len(text.len());
    }

    /// Whether the `added` bytes that now end `text`, beginning at `start`,
    /// are one grapheme cluster of their own.
    ///
    /// Clustering restarts at every cluster boundary, so reading back from
    /// the start of the cell before this one answers exactly what reading
    /// the whole line would. It is only an answer while that cell's extent
    /// is known, and where it is not this reports the cautious result.
    fn starts_its_own_cluster(&self, start: usize, added: usize) -> bool {
        if added == 0 {
            return false;
        }
        let previous = match self.last_cell_len {
            _ if start == 0 => 0,
            Some(len) => match start.checked_sub(len.get() as usize) {
                Some(previous) => previous,
                None => return false,
            },
            None => return false,
        };
        let mut graphemes = Graphemes::new(&self.text[previous..]);
        if start > previous && graphemes.next().map(str::len) != Some(start - previous) {
            return false;
        }
        graphemes.next().map(str::len) == Some(added) && graphemes.next().is_none()
    }

    pub fn to_cell_vec(&self) -> Vec<Cell> {
        let mut cells = vec![];

        for c in self.iter() {
            cells.push(c.as_cell());
            for _ in 1..c.width() {
                cells.push(Cell::blank_with_attrs(c.attrs().clone()));
            }
        }

        cells
    }

    pub fn from_cell_vec<'a>(hint: usize, iter: impl Iterator<Item = CellRef<'a>>) -> Self {
        let mut last_cluster: Option<Cluster> = None;
        let mut is_double_wide = FixedBitSet::with_capacity(hint);
        let mut text = String::new();
        let mut clusters = vec![];
        let mut any_double = false;
        let mut len = 0;
        let mut last_cell_width = None;
        let mut last_cell_len = 0;
        let mut cell_starts: Vec<usize> = vec![];

        for cell in iter {
            len += cell.width();
            last_cell_width = NonZeroU8::new(cell.width() as u8);
            last_cell_len = cell.str().len();

            if cell.width() > 1 {
                any_double = true;
                is_double_wide.set(cell.cell_index(), true);
            }

            cell_starts.push(text.len());
            text.push_str(cell.str());

            last_cluster = match last_cluster.take() {
                None => Some(Cluster {
                    cell_width: cell.width() as u16,
                    attrs: cell.attrs().clone(),
                }),
                Some(cluster) if cluster.attrs != *cell.attrs() => {
                    clusters.push(cluster);
                    Some(Cluster {
                        cell_width: cell.width() as u16,
                        attrs: cell.attrs().clone(),
                    })
                }
                Some(mut cluster) => {
                    // A run that would not fit its width has to be split,
                    // the same way appending one cell at a time splits it.
                    match cluster.cell_width.checked_add(cell.width() as u16) {
                        Some(cell_width) => {
                            cluster.cell_width = cell_width;
                            Some(cluster)
                        }
                        None => {
                            clusters.push(cluster);
                            Some(Cluster {
                                cell_width: cell.width() as u16,
                                attrs: cell.attrs().clone(),
                            })
                        }
                    }
                }
            };
        }

        if let Some(cluster) = last_cluster.take() {
            clusters.push(cluster);
        }

        let cell_starts = if grapheme_starts_match(&text, &cell_starts) {
            None
        } else {
            Some(Box::new(cell_starts))
        };

        Self {
            text,
            is_double_wide: if any_double {
                Some(Box::new(is_double_wide))
            } else {
                None
            },
            clusters,
            len: len.try_into().unwrap(),
            last_cell_width,
            last_cell_len: u16::try_from(last_cell_len).ok().and_then(NonZeroU16::new),
            cell_starts,
        }
    }

    pub fn len(&self) -> usize {
        self.len as usize
    }

    fn is_double_wide(&self, cell_index: usize) -> bool {
        match &self.is_double_wide {
            Some(bitset) => bitset.contains(cell_index),
            None => false,
        }
    }

    pub fn iter(&self) -> ClusterLineCellIter<'_> {
        let mut clusters = self.clusters.iter();
        let cluster = clusters.next();
        ClusterLineCellIter {
            text: match self.cell_starts.as_deref() {
                Some(starts) => CellTextIter::Recorded {
                    text: &self.text,
                    starts,
                    idx: 0,
                },
                None => CellTextIter::Graphemes(Graphemes::new(&self.text)),
            },
            clusters,
            cluster,
            idx: 0,
            cluster_total: 0,
            line: self,
        }
    }

    pub fn append_grapheme(&mut self, text: &str, cell_width: usize, attrs: CellAttributes) {
        let cell_width = cell_width as u16;
        let new_cluster = match self.clusters.last() {
            Some(cluster) => {
                if cluster.attrs != attrs {
                    true
                } else {
                    // If we overflow the max length of a run,
                    // then we need a new cluster
                    let (_, did_overflow) = cluster.cell_width.overflowing_add(cell_width);
                    did_overflow
                }
            }
            None => true,
        };
        let new_cell_index = self.len as usize;
        if new_cluster {
            self.clusters.push(Cluster { attrs, cell_width });
        } else if let Some(cluster) = self.clusters.last_mut() {
            cluster.cell_width += cell_width;
        }
        self.push_cell_text(text);

        if cell_width > 1 {
            let bitset = match self.is_double_wide.take() {
                Some(mut bitset) => {
                    bitset.grow(new_cell_index + 1);
                    bitset.set(new_cell_index, true);
                    bitset
                }
                None => {
                    let mut bitset = FixedBitSet::with_capacity(new_cell_index + 1);
                    bitset.set(new_cell_index, true);
                    Box::new(bitset)
                }
            };
            self.is_double_wide.replace(bitset);
        }
        self.last_cell_width = NonZeroU8::new(cell_width as u8);
        self.len += cell_width as u32;
    }

    pub fn append(&mut self, cell: Cell) {
        let cell_width = cell.width() as u16;
        let new_cluster = match self.clusters.last() {
            Some(cluster) => {
                if cluster.attrs != *cell.attrs() {
                    true
                } else {
                    // If we overflow the max length of a run,
                    // then we need a new cluster
                    let (_, did_overflow) = cluster.cell_width.overflowing_add(cell_width);
                    did_overflow
                }
            }
            None => true,
        };
        let new_cell_index = self.len as usize;
        if new_cluster {
            self.clusters.push(Cluster {
                attrs: (*cell.attrs()).clone(),
                cell_width,
            });
        } else if let Some(cluster) = self.clusters.last_mut() {
            cluster.cell_width += cell_width;
        }
        self.push_cell_text(cell.str());

        if cell_width > 1 {
            let bitset = match self.is_double_wide.take() {
                Some(mut bitset) => {
                    bitset.grow(new_cell_index + 1);
                    bitset.set(new_cell_index, true);
                    bitset
                }
                None => {
                    let mut bitset = FixedBitSet::with_capacity(new_cell_index + 1);
                    bitset.set(new_cell_index, true);
                    Box::new(bitset)
                }
            };
            self.is_double_wide.replace(bitset);
        }
        self.last_cell_width = NonZeroU8::new(cell_width as u8);
        self.len += cell_width as u32;
    }

    pub fn prune_trailing_blanks(&mut self) -> bool {
        let blank = CellAttributes::blank();

        // Count whole cells rather than trailing spaces: a cell can hold
        // more than the space that ends it, and removing only the space
        // would leave the rest of it behind.
        let (prunable, prune_from) = {
            let mut prunable = 0;
            let mut prune_from = self.text.len();
            let mut offset = 0;
            for cell in self.iter() {
                if cell.width() == 1 && cell.str() == " " && *cell.attrs() == blank {
                    if prunable == 0 {
                        prune_from = offset;
                    }
                    prunable += 1;
                } else {
                    prunable = 0;
                }
                offset += cell.str().len();
            }
            (prunable, prune_from)
        };

        if prunable == 0 {
            return false;
        }

        self.text.truncate(prune_from);
        if let Some(starts) = self.cell_starts.as_mut() {
            let keep = starts.len() - prunable;
            starts.truncate(keep);
        }
        self.len -= prunable as u32;

        // Take the same number of columns off the end of the attribute runs.
        let mut remaining = prunable;
        while remaining > 0 {
            match self.clusters.last_mut() {
                Some(cluster) => {
                    let taken = remaining.min(cluster.cell_width as usize);
                    cluster.cell_width -= taken as u16;
                    remaining -= taken;
                    if cluster.cell_width == 0 {
                        self.clusters.pop();
                    }
                }
                None => break,
            }
        }

        self.last_cell_width.take();
        let last_cell_len = self.iter().last().map_or(0, |cell| cell.str().len());
        self.set_last_cell_len(last_cell_len);

        true
    }

    fn compute_last_cell_width(&mut self) -> Option<NonZeroU8> {
        if self.last_cell_width.is_none() {
            if let Some(last_cell) = self.iter().last() {
                self.last_cell_width = NonZeroU8::new(last_cell.width() as u8);
            }
        }
        self.last_cell_width
    }

    pub fn set_last_cell_was_wrapped(&mut self, wrapped: bool) {
        if let Some(width) = self.compute_last_cell_width() {
            let width = width.get() as u16;
            if let Some(last_cluster) = self.clusters.last_mut() {
                let mut attrs = last_cluster.attrs.clone();
                attrs.set_wrapped(wrapped);

                if last_cluster.cell_width == width {
                    // Re-purpose final cluster
                    last_cluster.attrs = attrs;
                } else {
                    last_cluster.cell_width -= width;
                    self.clusters.push(Cluster {
                        cell_width: width,
                        attrs,
                    });
                }
            }
        }
    }
}

/// Yields the text of each cell of a clustered line, either by splitting the
/// line's text into grapheme clusters or by following the offsets the line
/// recorded because that split would not reproduce its cells.
enum CellTextIter<'a> {
    Graphemes(Graphemes<'a>),
    Recorded {
        text: &'a str,
        starts: &'a [usize],
        idx: usize,
    },
}

impl<'a> Iterator for CellTextIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        match self {
            Self::Graphemes(graphemes) => graphemes.next(),
            Self::Recorded { text, starts, idx } => {
                let start = *starts.get(*idx)?;
                let end = starts.get(*idx + 1).map_or(text.len(), |&next| next);
                *idx += 1;
                Some(&text[start..end])
            }
        }
    }
}

pub(crate) struct ClusterLineCellIter<'a> {
    text: CellTextIter<'a>,
    clusters: core::slice::Iter<'a, Cluster>,
    cluster: Option<&'a Cluster>,
    idx: usize,
    cluster_total: usize,
    line: &'a ClusteredLine,
}

impl<'a> Iterator for ClusterLineCellIter<'a> {
    type Item = CellRef<'a>;

    fn next(&mut self) -> Option<CellRef<'a>> {
        let text = self.text.next()?;

        let cell_index = self.idx;
        let width = if self.line.is_double_wide(cell_index) {
            2
        } else {
            1
        };
        self.idx += width;
        self.cluster_total += width;
        let attrs = &self.cluster.as_ref()?.attrs;

        if self.cluster_total >= self.cluster.as_ref()?.cell_width as usize {
            self.cluster = self.clusters.next();
            self.cluster_total = 0;
        }

        Some(CellRef::ClusterRef {
            cell_index,
            width,
            text,
            attrs,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn memory_usage() {
        assert_eq!(core::mem::size_of::<ClusteredLine>(), 72);
        assert_eq!(core::mem::size_of::<String>(), 24);
        assert_eq!(core::mem::size_of::<Vec<Cluster>>(), 24);
        assert_eq!(core::mem::size_of::<Option<Box<FixedBitSet>>>(), 8);
        assert_eq!(core::mem::size_of::<Option<NonZeroU8>>(), 1);
        assert_eq!(core::mem::size_of::<Option<NonZeroU16>>(), 2);
        assert_eq!(core::mem::size_of::<Option<Box<Vec<usize>>>>(), 8);
    }
}
