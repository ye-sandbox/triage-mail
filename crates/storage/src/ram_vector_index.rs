//! Zeroized-on-drop contiguous `int8[384]` vector and scoped eligibility index.
//!
//! INVARIANT: Safe Rust only (`#![deny(unsafe_code)]`). Never allocate or retain unzeroized
//! vector copies across reallocations, drops, lock transitions, or restores.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use rusqlite::{params, Connection, OptionalExtension};
use triage_ai::TriageCategory;
use zeroize::Zeroize;

use crate::sender_review::ReviewState;
use crate::{MailboxView, StorageError};

pub const RAM_VECTOR_DIM: usize = 384;
const PARALLEL_THRESHOLD: usize = 16_384;
const MAX_THREADS: usize = 8;

const FLAG_STARRED: u8 = 1 << 0;
const FLAG_HAS_SNOOZE: u8 = 1 << 1;
const FLAG_INBOX_HIDDEN: u8 = 1 << 2;
const REVIEW_SHIFT: u8 = 3;
const REVIEW_MASK: u8 = 0b111;

const REVIEW_NONE: u8 = 0;
const REVIEW_KNOWN: u8 = 1;
const REVIEW_PENDING: u8 = 2;
const REVIEW_ACCEPTED: u8 = 3;
const REVIEW_DECLINED: u8 = 4;

const CATEGORY_UNKNOWN: u8 = u8::MAX;

#[derive(Debug)]
pub(crate) enum RamVectorState {
    Uninitialized,
    Hydrating,
    Ready(Box<ReadyRamIndex>),
}

#[derive(Debug)]
pub(crate) struct ReadyRamIndex {
    pub(crate) index: RamVectorIndex,
    pub(crate) synced_total_changes: i64,
    pub(crate) synced_data_version: i64,
}

#[derive(Debug, Clone, Copy)]
pub enum RamScopeFilter<'a> {
    Inbox {
        folder: &'a str,
        now_ms: i64,
        apply_review_hide: bool,
    },
    Starred,
    Snoozed {
        now_ms: i64,
    },
    Folder {
        folder: &'a str,
    },
}

#[derive(Debug, Clone, Copy, Default)]
pub struct RamQueryFilter<'a> {
    pub account_id: Option<&'a str>,
    pub scope: Option<RamScopeFilter<'a>>,
    pub category: Option<TriageCategory>,
    pub review_state: Option<ReviewState>,
}

#[derive(Debug, Clone, Copy)]
enum SnoozePredicate {
    Any,
    NotActive(i64),
    Active(i64),
}

#[derive(Debug, Clone, Copy)]
struct PreparedFilter {
    target_account: Option<u32>,
    target_folder: Option<u32>,
    check_starred: bool,
    snooze: SnoozePredicate,
    apply_review_hide: bool,
    target_review: Option<u8>,
    target_category: Option<u8>,
}

/// Zeroized contiguous SoA arena storing `int8[384]` vectors and scoped eligibility columns.
#[derive(Debug, Default)]
pub struct RamVectorIndex {
    len: usize,
    cap: usize,
    vectors: Vec<i8>,
    norm_sq: Vec<u32>,
    rowids: Vec<i64>,
    message_ids: Vec<String>,
    account_idx: Vec<u32>,
    flags: Vec<u8>,
    category: Vec<u8>,
    snooze_until: Vec<i64>,
    folder_mask: Vec<u64>,
    overflow_folders: HashMap<usize, Vec<u32>>,
    id_to_slot: HashMap<String, usize>,
    rowid_to_slot: HashMap<i64, usize>,
    accounts: Vec<String>,
    account_map: HashMap<String, u32>,
    folders: Vec<String>,
    folder_map: HashMap<String, u32>,
}

impl Drop for RamVectorIndex {
    fn drop(&mut self) {
        self.clear();
    }
}

impl RamVectorIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(slots: usize) -> Self {
        let cap_elems = slots.saturating_mul(RAM_VECTOR_DIM);
        Self {
            len: 0,
            cap: slots,
            vectors: Vec::with_capacity(cap_elems),
            norm_sq: Vec::with_capacity(slots),
            rowids: Vec::with_capacity(slots),
            message_ids: Vec::with_capacity(slots),
            account_idx: Vec::with_capacity(slots),
            flags: Vec::with_capacity(slots),
            category: Vec::with_capacity(slots),
            snooze_until: Vec::with_capacity(slots),
            folder_mask: Vec::with_capacity(slots),
            overflow_folders: HashMap::new(),
            id_to_slot: HashMap::with_capacity(slots),
            rowid_to_slot: HashMap::with_capacity(slots),
            accounts: Vec::new(),
            account_map: HashMap::new(),
            folders: Vec::new(),
            folder_map: HashMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Approximate heap bytes currently held by vectors + eligibility metadata.
    pub fn vector_bytes_capacity(&self) -> usize {
        self.vectors.capacity()
    }

    /// Returns `(vector_bytes, eligibility_and_lookup_bytes)` allocated on the heap.
    pub fn allocated_bytes(&self) -> (usize, usize) {
        let vec_bytes = self.vectors.capacity();
        let col_bytes = self.norm_sq.capacity() * std::mem::size_of::<u32>()
            + self.rowids.capacity() * std::mem::size_of::<i64>()
            + self.message_ids.capacity() * std::mem::size_of::<String>()
            + self.message_ids.iter().map(|s| s.capacity()).sum::<usize>()
            + self.account_idx.capacity() * std::mem::size_of::<u32>()
            + self.flags.capacity()
            + self.category.capacity()
            + self.snooze_until.capacity() * std::mem::size_of::<i64>()
            + self.folder_mask.capacity() * std::mem::size_of::<u64>()
            + self
                .overflow_folders
                .values()
                .map(|v| v.capacity() * std::mem::size_of::<u32>())
                .sum::<usize>()
            + self.id_to_slot.capacity()
                * (std::mem::size_of::<String>() + std::mem::size_of::<usize>())
            + self.rowid_to_slot.capacity()
                * (std::mem::size_of::<i64>() + std::mem::size_of::<usize>());
        (vec_bytes, col_bytes)
    }

    /// Raw slice of the packed `int8` vector buffer (intended for zeroize tests).
    #[cfg(test)]
    pub(crate) fn raw_vectors_slice(&self) -> &[i8] {
        &self.vectors
    }

    /// Explicitly zeroes every vector, ID, interned symbol, and eligibility buffer.
    pub fn clear(&mut self) {
        self.vectors.zeroize();
        self.vectors.clear();
        self.vectors.shrink_to_fit();
        self.norm_sq.zeroize();
        self.norm_sq.clear();
        self.rowids.zeroize();
        self.rowids.clear();
        for id in &mut self.message_ids {
            id.zeroize();
        }
        self.message_ids.clear();
        self.account_idx.zeroize();
        self.account_idx.clear();
        self.flags.zeroize();
        self.flags.clear();
        self.category.zeroize();
        self.category.clear();
        self.snooze_until.zeroize();
        self.snooze_until.clear();
        self.folder_mask.zeroize();
        self.folder_mask.clear();
        for (_, mut list) in self.overflow_folders.drain() {
            list.zeroize();
        }
        for (mut key, mut val) in self.id_to_slot.drain() {
            key.zeroize();
            val.zeroize();
        }
        for (mut key, mut val) in self.rowid_to_slot.drain() {
            key.zeroize();
            val.zeroize();
        }
        for acc in &mut self.accounts {
            acc.zeroize();
        }
        self.accounts.clear();
        for (mut key, mut val) in self.account_map.drain() {
            key.zeroize();
            val.zeroize();
        }
        for folder in &mut self.folders {
            folder.zeroize();
        }
        self.folders.clear();
        for (mut key, mut val) in self.folder_map.drain() {
            key.zeroize();
            val.zeroize();
        }
        self.len = 0;
        self.cap = 0;
    }

    fn ensure_vector_capacity(&mut self, required_slots: usize) {
        if required_slots <= self.cap {
            return;
        }
        let new_cap = required_slots.max(self.cap.saturating_mul(2)).max(64);
        let mut next = Vec::with_capacity(new_cap.saturating_mul(RAM_VECTOR_DIM));
        next.extend_from_slice(&self.vectors);
        // INVARIANT: Zero previous contiguous vector buffer before freeing old heap block.
        self.vectors.zeroize();
        self.vectors = next;
        self.cap = new_cap;
    }

    fn intern_account(&mut self, account_id: &str) -> u32 {
        if let Some(&idx) = self.account_map.get(account_id) {
            return idx;
        }
        let idx = self.accounts.len() as u32;
        self.accounts.push(account_id.to_string());
        self.account_map.insert(account_id.to_string(), idx);
        idx
    }

    fn intern_folder(&mut self, folder: &str) -> u32 {
        if let Some(&idx) = self.folder_map.get(folder) {
            return idx;
        }
        let idx = self.folders.len() as u32;
        self.folders.push(folder.to_string());
        self.folder_map.insert(folder.to_string(), idx);
        idx
    }

    fn add_folder_to_slot(&mut self, slot: usize, folder_idx: u32) {
        if folder_idx < 64 {
            self.folder_mask[slot] |= 1_u64 << folder_idx;
        } else {
            let list = self.overflow_folders.entry(slot).or_default();
            if !list.contains(&folder_idx) {
                list.push(folder_idx);
            }
        }
    }

    fn clear_slot_folders(&mut self, slot: usize) {
        self.folder_mask[slot] = 0;
        if let Some(mut list) = self.overflow_folders.remove(&slot) {
            list.zeroize();
        }
    }

    #[inline]
    fn slot_has_folder(&self, slot: usize, folder_idx: u32) -> bool {
        if folder_idx < 64 {
            (self.folder_mask[slot] & (1_u64 << folder_idx)) != 0
        } else {
            self.overflow_folders
                .get(&slot)
                .is_some_and(|list| list.contains(&folder_idx))
        }
    }

    /// Inserts or updates a slot in the arena.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn upsert_slot(
        &mut self,
        rowid: i64,
        message_id: &str,
        account_id: &str,
        vector: &[i8],
        is_starred: bool,
        snooze_until: Option<i64>,
        category_str: Option<&str>,
        review_state: Option<ReviewState>,
        inbox_hidden: bool,
        effective_folders: &[String],
    ) -> Result<(), StorageError> {
        if vector.len() != RAM_VECTOR_DIM {
            return Err(StorageError::InvalidSearchQuery(
                "vector must contain 384 int8 dimensions".into(),
            ));
        }
        let norm_sq = compute_norm_sq(vector);
        let acc_idx = self.intern_account(account_id);
        let flags = pack_flags(
            is_starred,
            snooze_until.is_some(),
            inbox_hidden,
            review_state,
        );
        let cat_code = category_to_code(category_str);
        let snooze_val = snooze_until.unwrap_or(0);

        if let Some(&slot) = self.id_to_slot.get(message_id) {
            let old_rowid = self.rowids[slot];
            if old_rowid != rowid {
                self.rowid_to_slot.remove(&old_rowid);
                self.rowids[slot] = rowid;
                self.rowid_to_slot.insert(rowid, slot);
            }
            let start = slot * RAM_VECTOR_DIM;
            self.vectors[start..start + RAM_VECTOR_DIM].copy_from_slice(vector);
            self.norm_sq[slot] = norm_sq;
            self.account_idx[slot] = acc_idx;
            self.flags[slot] = flags;
            self.category[slot] = cat_code;
            self.snooze_until[slot] = snooze_val;
            self.clear_slot_folders(slot);
            for folder in effective_folders {
                let f_idx = self.intern_folder(folder);
                self.add_folder_to_slot(slot, f_idx);
            }
            return Ok(());
        }

        self.ensure_vector_capacity(self.len + 1);
        let slot = self.len;
        self.len += 1;
        self.vectors.extend_from_slice(vector);
        self.norm_sq.push(norm_sq);
        self.rowids.push(rowid);
        self.message_ids.push(message_id.to_string());
        self.account_idx.push(acc_idx);
        self.flags.push(flags);
        self.category.push(cat_code);
        self.snooze_until.push(snooze_val);
        self.folder_mask.push(0);
        for folder in effective_folders {
            let f_idx = self.intern_folder(folder);
            self.add_folder_to_slot(slot, f_idx);
        }
        self.id_to_slot.insert(message_id.to_string(), slot);
        self.rowid_to_slot.insert(rowid, slot);
        Ok(())
    }

    pub(crate) fn remove_slot(&mut self, message_id: &str) {
        let Some(slot) = self.id_to_slot.remove(message_id) else {
            return;
        };
        let last = self.len - 1;
        let removed_rowid = self.rowids[slot];
        self.rowid_to_slot.remove(&removed_rowid);

        if slot != last {
            let dst = slot * RAM_VECTOR_DIM;
            let src = last * RAM_VECTOR_DIM;
            self.vectors.copy_within(src..src + RAM_VECTOR_DIM, dst);
            self.norm_sq[slot] = self.norm_sq[last];
            self.rowids[slot] = self.rowids[last];
            self.message_ids.swap(slot, last);
            self.account_idx[slot] = self.account_idx[last];
            self.flags[slot] = self.flags[last];
            self.category[slot] = self.category[last];
            self.snooze_until[slot] = self.snooze_until[last];
            self.folder_mask[slot] = self.folder_mask[last];

            if let Some(mut old_over) = self.overflow_folders.remove(&slot) {
                old_over.zeroize();
            }
            if let Some(last_over) = self.overflow_folders.remove(&last) {
                self.overflow_folders.insert(slot, last_over);
            }

            let moved_id = &self.message_ids[slot];
            let moved_rowid = self.rowids[slot];
            self.id_to_slot.insert(moved_id.clone(), slot);
            self.rowid_to_slot.insert(moved_rowid, slot);
        } else if let Some(mut old_over) = self.overflow_folders.remove(&slot) {
            old_over.zeroize();
        }

        // INVARIANT: Zero vacated tail bytes before popping/truncating.
        let tail_start = last * RAM_VECTOR_DIM;
        self.vectors[tail_start..tail_start + RAM_VECTOR_DIM].zeroize();
        self.vectors.truncate(tail_start);
        self.norm_sq[last] = 0;
        self.norm_sq.pop();
        self.rowids[last] = 0;
        self.rowids.pop();
        if let Some(mut old_id) = self.message_ids.pop() {
            old_id.zeroize();
        }
        self.account_idx[last] = 0;
        self.account_idx.pop();
        self.flags[last] = 0;
        self.flags.pop();
        self.category[last] = 0;
        self.category.pop();
        self.snooze_until[last] = 0;
        self.snooze_until.pop();
        self.folder_mask[last] = 0;
        self.folder_mask.pop();
        self.len = last;
    }

    pub(crate) fn contains_id(&self, message_id: &str) -> bool {
        self.id_to_slot.contains_key(message_id)
    }

    pub(crate) fn clear_snooze_if_present(&mut self, message_id: &str) {
        if let Some(&slot) = self.id_to_slot.get(message_id) {
            self.flags[slot] &= !FLAG_HAS_SNOOZE;
            self.snooze_until[slot] = 0;
        }
    }

    pub(crate) fn set_category_if_present(&mut self, message_id: &str, cat: Option<&str>) {
        if let Some(&slot) = self.id_to_slot.get(message_id) {
            self.category[slot] = category_to_code(cat);
        }
    }

    pub(crate) fn set_review_for_account_inactive(&mut self, account_id: &str) {
        let Some(&acc_idx) = self.account_map.get(account_id) else {
            return;
        };
        for slot in 0..self.len {
            if self.account_idx[slot] == acc_idx {
                self.flags[slot] &= !(FLAG_INBOX_HIDDEN | (REVIEW_MASK << REVIEW_SHIFT));
            }
        }
    }

    pub(crate) fn set_review_state_if_present(
        &mut self,
        message_id: &str,
        review_state: Option<ReviewState>,
        inbox_hidden: bool,
    ) {
        if let Some(&slot) = self.id_to_slot.get(message_id) {
            let base = self.flags[slot] & (FLAG_STARRED | FLAG_HAS_SNOOZE);
            let hide_bit = if inbox_hidden { FLAG_INBOX_HIDDEN } else { 0 };
            let rev_bits = (review_to_code(review_state) & REVIEW_MASK) << REVIEW_SHIFT;
            self.flags[slot] = base | hide_bit | rev_bits;
        }
    }

    fn prepare_filter(&self, filter: &RamQueryFilter<'_>) -> Option<PreparedFilter> {
        let target_account = match filter.account_id {
            Some(acc) if !acc.is_empty() => Some(*self.account_map.get(acc)?),
            _ => None,
        };
        let mut target_folder = None;
        let mut check_starred = false;
        let mut snooze = SnoozePredicate::Any;
        let mut apply_review_hide = false;

        if let Some(scope) = filter.scope {
            match scope {
                RamScopeFilter::Inbox {
                    folder,
                    now_ms,
                    apply_review_hide: hide,
                } => {
                    target_folder = Some(*self.folder_map.get(folder)?);
                    snooze = SnoozePredicate::NotActive(now_ms);
                    apply_review_hide = hide;
                }
                RamScopeFilter::Starred => {
                    check_starred = true;
                }
                RamScopeFilter::Snoozed { now_ms } => {
                    snooze = SnoozePredicate::Active(now_ms);
                }
                RamScopeFilter::Folder { folder } => {
                    target_folder = Some(*self.folder_map.get(folder)?);
                }
            }
        }

        let target_category = filter
            .category
            .map(|cat| category_to_code(Some(cat.as_str())));
        let target_review = filter.review_state.map(|rev| review_to_code(Some(rev)));

        Some(PreparedFilter {
            target_account,
            target_folder,
            check_starred,
            snooze,
            apply_review_hide,
            target_review,
            target_category,
        })
    }

    #[inline]
    fn slot_matches(&self, slot: usize, f: &PreparedFilter) -> bool {
        if let Some(acc) = f.target_account {
            if self.account_idx[slot] != acc {
                return false;
            }
        }
        let flags = self.flags[slot];
        if f.check_starred && (flags & FLAG_STARRED) == 0 {
            return false;
        }
        match f.snooze {
            SnoozePredicate::Any => {}
            SnoozePredicate::NotActive(now_ms) => {
                if (flags & FLAG_HAS_SNOOZE) != 0 && self.snooze_until[slot] > now_ms {
                    return false;
                }
            }
            SnoozePredicate::Active(now_ms) => {
                if (flags & FLAG_HAS_SNOOZE) == 0 || self.snooze_until[slot] <= now_ms {
                    return false;
                }
            }
        }
        if f.apply_review_hide && (flags & FLAG_INBOX_HIDDEN) != 0 {
            return false;
        }
        if let Some(rev) = f.target_review {
            if ((flags >> REVIEW_SHIFT) & REVIEW_MASK) != rev {
                return false;
            }
        }
        if let Some(cat) = f.target_category {
            if self.category[slot] != cat {
                return false;
            }
        }
        if let Some(folder_idx) = f.target_folder {
            if !self.slot_has_folder(slot, folder_idx) {
                return false;
            }
        }
        true
    }

    /// Exact cosine distance ranking over RAM eligibility columns.
    ///
    /// INVARIANT: Preserves `sqlite-vec` 0.1.9 exact `f32` cosine distances and `k-021` NULL-first
    /// ordering (`f64::NEG_INFINITY` for zero-norm stored or query vectors) with `message_id` ASC ties.
    pub fn rank_filtered(
        &self,
        query: &[i8],
        filter: &RamQueryFilter<'_>,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<String>, StorageError> {
        let query_arr: &[i8; RAM_VECTOR_DIM] = query.try_into().map_err(|_| {
            StorageError::InvalidSearchQuery(
                "query embedding must contain 384 int8 dimensions".into(),
            )
        })?;
        if limit == 0 || self.len == 0 {
            return Ok(Vec::new());
        }
        let Some(end) = offset.checked_add(limit) else {
            return Err(StorageError::InvalidSearchQuery(
                "pagination exceeds integer bounds".into(),
            ));
        };
        let Some(prepared) = self.prepare_filter(filter) else {
            return Ok(Vec::new());
        };
        let query_norm_sq = compute_norm_sq(query_arr);
        let query_norm_sqrt = f64::from(query_norm_sq).sqrt();

        if end >= self.len {
            let mut matches: Vec<(f64, usize)> = Vec::new();
            for slot in 0..self.len {
                if self.slot_matches(slot, &prepared) {
                    let dist = self.slot_distance(slot, query_arr, query_norm_sq, query_norm_sqrt);
                    matches.push((dist, slot));
                }
            }
            matches.sort_unstable_by(|a, b| {
                a.0.total_cmp(&b.0)
                    .then_with(|| self.message_ids[a.1].cmp(&self.message_ids[b.1]))
            });
            return Ok(matches
                .into_iter()
                .skip(offset)
                .take(limit)
                .map(|(_, slot)| self.message_ids[slot].clone())
                .collect());
        }

        let threads = if self.len >= PARALLEL_THRESHOLD {
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1)
                .clamp(1, MAX_THREADS)
        } else {
            1
        };

        if threads == 1 {
            let heap = self.scan_range_heap(
                0..self.len,
                &prepared,
                query_arr,
                query_norm_sq,
                query_norm_sqrt,
                end,
            );
            return Ok(self.extract_page_from_heap(heap, offset, limit));
        }

        let chunk_size = self.len.div_ceil(threads);
        let mut merged: BinaryHeap<WorstFirst<'_>> = BinaryHeap::with_capacity(end + 1);
        std::thread::scope(|scope| {
            let mut handles = Vec::with_capacity(threads);
            let mut start = 0;
            while start < self.len {
                let stop = (start + chunk_size).min(self.len);
                let range = start..stop;
                let this = self;
                handles.push(scope.spawn(move || {
                    this.scan_range_heap(
                        range,
                        &prepared,
                        query_arr,
                        query_norm_sq,
                        query_norm_sqrt,
                        end,
                    )
                }));
                start = stop;
            }
            for handle in handles {
                if let Ok(local_heap) = handle.join() {
                    for entry in local_heap {
                        push_bounded(&mut merged, entry, end);
                    }
                }
            }
        });
        Ok(self.extract_page_from_heap(merged, offset, limit))
    }

    /// Exact cosine distance ranking over an arbitrary SQLite-filtered slice of `messages.rowid`.
    pub(crate) fn rank_rowids(
        &self,
        query: &[i8],
        rowids: &[i64],
        offset: usize,
        limit: usize,
    ) -> Result<Vec<String>, StorageError> {
        let query_arr: &[i8; RAM_VECTOR_DIM] = query.try_into().map_err(|_| {
            StorageError::InvalidSearchQuery(
                "query embedding must contain 384 int8 dimensions".into(),
            )
        })?;
        if limit == 0 || rowids.is_empty() || self.len == 0 {
            return Ok(Vec::new());
        }
        let Some(end) = offset.checked_add(limit) else {
            return Err(StorageError::InvalidSearchQuery(
                "pagination exceeds integer bounds".into(),
            ));
        };
        let query_norm_sq = compute_norm_sq(query_arr);
        let query_norm_sqrt = f64::from(query_norm_sq).sqrt();

        if end >= rowids.len() {
            let mut matches: Vec<(f64, usize)> = Vec::with_capacity(rowids.len().min(self.len));
            for &rowid in rowids {
                if let Some(&slot) = self.rowid_to_slot.get(&rowid) {
                    let dist = self.slot_distance(slot, query_arr, query_norm_sq, query_norm_sqrt);
                    matches.push((dist, slot));
                }
            }
            matches.sort_unstable_by(|a, b| {
                a.0.total_cmp(&b.0)
                    .then_with(|| self.message_ids[a.1].cmp(&self.message_ids[b.1]))
            });
            return Ok(matches
                .into_iter()
                .skip(offset)
                .take(limit)
                .map(|(_, slot)| self.message_ids[slot].clone())
                .collect());
        }

        let threads = if rowids.len() >= PARALLEL_THRESHOLD {
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1)
                .clamp(1, MAX_THREADS)
        } else {
            1
        };

        if threads == 1 {
            let heap =
                self.scan_rowids_heap(rowids, query_arr, query_norm_sq, query_norm_sqrt, end);
            return Ok(self.extract_page_from_heap(heap, offset, limit));
        }

        let chunk_size = rowids.len().div_ceil(threads);
        let mut merged: BinaryHeap<WorstFirst<'_>> = BinaryHeap::with_capacity(end + 1);
        std::thread::scope(|scope| {
            let mut handles = Vec::with_capacity(threads);
            for chunk in rowids.chunks(chunk_size) {
                let this = self;
                handles.push(scope.spawn(move || {
                    this.scan_rowids_heap(chunk, query_arr, query_norm_sq, query_norm_sqrt, end)
                }));
            }
            for handle in handles {
                if let Ok(local_heap) = handle.join() {
                    for entry in local_heap {
                        push_bounded(&mut merged, entry, end);
                    }
                }
            }
        });
        Ok(self.extract_page_from_heap(merged, offset, limit))
    }

    #[inline]
    fn slot_distance(
        &self,
        slot: usize,
        query: &[i8; RAM_VECTOR_DIM],
        query_norm_sq: u32,
        query_norm_sqrt: f64,
    ) -> f64 {
        let doc_norm_sq = self.norm_sq[slot];
        if query_norm_sq == 0 || doc_norm_sq == 0 {
            return f64::NEG_INFINITY;
        }
        let start = slot * RAM_VECTOR_DIM;
        let doc: &[i8; RAM_VECTOR_DIM] =
            match self.vectors[start..start + RAM_VECTOR_DIM].try_into() {
                Ok(slice) => slice,
                Err(_) => return f64::NEG_INFINITY,
            };
        let dot = dot_i8_384(doc, query);
        let denom = f64::from(doc_norm_sq).sqrt() * query_norm_sqrt;
        // INVARIANT: Match sqlite-vec 0.1.9 distance_cosine_int8 float32 rounding widened to double.
        let dist_f32 = (1.0_f64 - f64::from(dot) / denom) as f32;
        f64::from(dist_f32)
    }

    fn scan_range_heap(
        &self,
        range: std::ops::Range<usize>,
        filter: &PreparedFilter,
        query: &[i8; RAM_VECTOR_DIM],
        query_norm_sq: u32,
        query_norm_sqrt: f64,
        end: usize,
    ) -> BinaryHeap<WorstFirst<'_>> {
        let mut heap = BinaryHeap::with_capacity(end + 1);
        for slot in range {
            if !self.slot_matches(slot, filter) {
                continue;
            }
            let dist = self.slot_distance(slot, query, query_norm_sq, query_norm_sqrt);
            let entry = WorstFirst {
                dist,
                id: &self.message_ids[slot],
                slot,
            };
            push_bounded(&mut heap, entry, end);
        }
        heap
    }

    fn scan_rowids_heap(
        &self,
        rowids: &[i64],
        query: &[i8; RAM_VECTOR_DIM],
        query_norm_sq: u32,
        query_norm_sqrt: f64,
        end: usize,
    ) -> BinaryHeap<WorstFirst<'_>> {
        let mut heap = BinaryHeap::with_capacity(end + 1);
        for &rowid in rowids {
            let Some(&slot) = self.rowid_to_slot.get(&rowid) else {
                continue;
            };
            let dist = self.slot_distance(slot, query, query_norm_sq, query_norm_sqrt);
            let entry = WorstFirst {
                dist,
                id: &self.message_ids[slot],
                slot,
            };
            push_bounded(&mut heap, entry, end);
        }
        heap
    }

    fn extract_page_from_heap(
        &self,
        heap: BinaryHeap<WorstFirst<'_>>,
        offset: usize,
        limit: usize,
    ) -> Vec<String> {
        let mut sorted = heap.into_vec();
        sorted.sort_unstable_by(|a, b| a.dist.total_cmp(&b.dist).then_with(|| a.id.cmp(b.id)));
        sorted
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(|entry| entry.id.to_string())
            .collect()
    }
}

/// Max-heap ordering: largest distance at the top, breaking ties by largest `message_id`.
#[derive(Debug, Clone, Copy)]
struct WorstFirst<'a> {
    dist: f64,
    id: &'a str,
    #[allow(dead_code)]
    slot: usize,
}

impl PartialEq for WorstFirst<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.dist.total_cmp(&other.dist) == Ordering::Equal && self.id == other.id
    }
}

impl Eq for WorstFirst<'_> {}

impl PartialOrd for WorstFirst<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for WorstFirst<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.dist
            .total_cmp(&other.dist)
            .then_with(|| self.id.cmp(other.id))
    }
}

#[inline]
fn push_bounded<'a>(heap: &mut BinaryHeap<WorstFirst<'a>>, entry: WorstFirst<'a>, cap: usize) {
    if heap.len() < cap {
        heap.push(entry);
    } else if let Some(mut top) = heap.peek_mut() {
        if entry < *top {
            *top = entry;
        }
    }
}

#[inline]
fn compute_norm_sq(vec: &[i8]) -> u32 {
    let mut sum: u32 = 0;
    for &x in vec {
        let xi = i32::from(x);
        sum += (xi * xi) as u32;
    }
    sum
}

/// 8-lane unrolled `i32` dot product over `int8[384]`. Max magnitude is `384 * 128^2 = 6,291,456`.
#[inline]
fn dot_i8_384(a: &[i8; RAM_VECTOR_DIM], b: &[i8; RAM_VECTOR_DIM]) -> i32 {
    let mut s0 = 0i32;
    let mut s1 = 0i32;
    let mut s2 = 0i32;
    let mut s3 = 0i32;
    let mut s4 = 0i32;
    let mut s5 = 0i32;
    let mut s6 = 0i32;
    let mut s7 = 0i32;
    for (ca, cb) in a.as_chunks::<8>().0.iter().zip(b.as_chunks::<8>().0.iter()) {
        s0 += i32::from(ca[0]) * i32::from(cb[0]);
        s1 += i32::from(ca[1]) * i32::from(cb[1]);
        s2 += i32::from(ca[2]) * i32::from(cb[2]);
        s3 += i32::from(ca[3]) * i32::from(cb[3]);
        s4 += i32::from(ca[4]) * i32::from(cb[4]);
        s5 += i32::from(ca[5]) * i32::from(cb[5]);
        s6 += i32::from(ca[6]) * i32::from(cb[6]);
        s7 += i32::from(ca[7]) * i32::from(cb[7]);
    }
    (s0 + s1) + (s2 + s3) + (s4 + s5) + (s6 + s7)
}

#[inline]
fn pack_flags(
    is_starred: bool,
    has_snooze: bool,
    inbox_hidden: bool,
    review_state: Option<ReviewState>,
) -> u8 {
    let mut flags = 0u8;
    if is_starred {
        flags |= FLAG_STARRED;
    }
    if has_snooze {
        flags |= FLAG_HAS_SNOOZE;
    }
    if inbox_hidden {
        flags |= FLAG_INBOX_HIDDEN;
    }
    flags |= (review_to_code(review_state) & REVIEW_MASK) << REVIEW_SHIFT;
    flags
}

#[inline]
fn category_to_code(cat: Option<&str>) -> u8 {
    match cat.and_then(TriageCategory::parse) {
        Some(TriageCategory::Personal) => 0,
        Some(TriageCategory::Security) => 1,
        Some(TriageCategory::Financial) => 2,
        Some(TriageCategory::Shopping) => 3,
        Some(TriageCategory::Newsletter) => 4,
        Some(TriageCategory::Notifications) => 5,
        None => CATEGORY_UNKNOWN,
    }
}

#[inline]
fn review_to_code(state: Option<ReviewState>) -> u8 {
    match state {
        None => REVIEW_NONE,
        Some(ReviewState::Known) => REVIEW_KNOWN,
        Some(ReviewState::Pending) => REVIEW_PENDING,
        Some(ReviewState::Accepted) => REVIEW_ACCEPTED,
        Some(ReviewState::Declined) => REVIEW_DECLINED,
    }
}

#[inline]
pub(crate) fn parse_review_state(value: &str) -> Option<ReviewState> {
    match value {
        "Known" => Some(ReviewState::Known),
        "Pending" => Some(ReviewState::Pending),
        "Accepted" => Some(ReviewState::Accepted),
        "Declined" => Some(ReviewState::Declined),
        _ => None,
    }
}

pub(crate) fn scope_to_ram_filter<'a>(
    view: &MailboxView,
    resolved_folder: &'a str,
    now_ms: i64,
    apply_review_hide: bool,
) -> RamScopeFilter<'a> {
    match view {
        MailboxView::Inbox => RamScopeFilter::Inbox {
            folder: resolved_folder,
            now_ms,
            apply_review_hide,
        },
        MailboxView::Starred => RamScopeFilter::Starred,
        MailboxView::Snoozed => RamScopeFilter::Snoozed { now_ms },
        MailboxView::Folder(_) => RamScopeFilter::Folder {
            folder: resolved_folder,
        },
    }
}

pub(crate) fn read_conn_versions(conn: &Connection) -> Result<(i64, i64), StorageError> {
    let tc: i64 = conn
        .prepare_cached("SELECT total_changes()")
        .map_err(|e| StorageError::Query(e.to_string()))?
        .query_row([], |r| r.get(0))
        .map_err(|e| StorageError::Query(e.to_string()))?;
    let dv: i64 = conn
        .prepare_cached("PRAGMA data_version")
        .map_err(|e| StorageError::Query(e.to_string()))?
        .query_row([], |r| r.get(0))
        .map_err(|e| StorageError::Query(e.to_string()))?;
    Ok((tc, dv))
}

/// Builds a fresh `RamVectorIndex` from a consistent read snapshot on `conn`.
///
/// INVARIANT: Joins `vec_messages` with `messages` so orphan `vec_messages` rows never enter
/// the index and unembedded messages are naturally absent.
pub(crate) fn build_from_conn(conn: &Connection) -> Result<RamVectorIndex, StorageError> {
    let approx_count: usize = conn
        .query_row("SELECT count(*) FROM vec_messages", [], |r| r.get(0))
        .map_err(|e| StorageError::Query(e.to_string()))?;
    let mut index = RamVectorIndex::with_capacity(approx_count);
    let mut pending_overrides: Vec<Option<u32>> = Vec::with_capacity(approx_count);

    {
        let sql = format!(
            "SELECT
                m.rowid,
                m.id,
                m.account_id,
                v.embedding_int8,
                m.is_starred,
                m.snooze_until_epoch_ms,
                o.folder,
                {} AS effective_category,
                CASE
                    WHEN p.enabled = 1 AND p.initializing = 0 THEN c.state
                    ELSE NULL
                END AS review_state,
                CASE
                    WHEN p.enabled = 1 AND p.initializing = 0 AND p.hide_in_inbox = 1
                         AND c.state IN ('Pending', 'Declined') THEN 1
                    ELSE 0
                END AS inbox_hidden
             FROM vec_messages v
             JOIN messages m ON m.id = v.message_id
             LEFT JOIN message_folder_overrides o ON o.message_id = m.id
             LEFT JOIN sender_review_messages r ON r.message_id = m.id
             LEFT JOIN sender_review_choices c ON c.account_id = r.account_id AND c.sender = r.sender
             LEFT JOIN sender_review_preferences p ON p.account_id = r.account_id",
            crate::search::CATEGORY_EFFECTIVE
        );
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| StorageError::Query(e.to_string()))?;
        let mut rows = stmt
            .query([])
            .map_err(|e| StorageError::Query(e.to_string()))?;
        let mut vec_buf = vec![0i8; RAM_VECTOR_DIM];
        while let Some(row) = rows
            .next()
            .map_err(|e| StorageError::Query(e.to_string()))?
        {
            let rowid: i64 = row.get(0).map_err(|e| StorageError::Query(e.to_string()))?;
            let id: String = row.get(1).map_err(|e| StorageError::Query(e.to_string()))?;
            let account_id: String = row.get(2).map_err(|e| StorageError::Query(e.to_string()))?;
            let mut blob: Vec<u8> = row.get(3).map_err(|e| StorageError::Query(e.to_string()))?;
            if blob.len() != RAM_VECTOR_DIM {
                blob.zeroize();
                vec_buf.zeroize();
                return Err(StorageError::Query(
                    "corrupted vec_messages dimension in RAM hydration".into(),
                ));
            }
            for (dst, &src) in vec_buf.iter_mut().zip(blob.iter()) {
                *dst = src as i8;
            }
            blob.zeroize();
            let is_starred: bool = row
                .get::<_, i64>(4)
                .map_err(|e| StorageError::Query(e.to_string()))?
                != 0;
            let snooze_until: Option<i64> =
                row.get(5).map_err(|e| StorageError::Query(e.to_string()))?;
            let override_folder: Option<String> =
                row.get(6).map_err(|e| StorageError::Query(e.to_string()))?;
            let effective_cat: Option<String> =
                row.get(7).map_err(|e| StorageError::Query(e.to_string()))?;
            let review_str: Option<String> =
                row.get(8).map_err(|e| StorageError::Query(e.to_string()))?;
            let inbox_hidden: bool = row
                .get::<_, i64>(9)
                .map_err(|e| StorageError::Query(e.to_string()))?
                != 0;

            let ovr_idx = override_folder.as_deref().map(|f| index.intern_folder(f));
            let existing_slot = index.id_to_slot.get(&id).copied();

            index.upsert_slot(
                rowid,
                &id,
                &account_id,
                &vec_buf,
                is_starred,
                snooze_until,
                effective_cat.as_deref(),
                review_str.as_deref().and_then(parse_review_state),
                inbox_hidden,
                &[],
            )?;
            if let Some(slot) = existing_slot {
                pending_overrides[slot] = ovr_idx;
            } else {
                pending_overrides.push(ovr_idx);
            }
        }
        vec_buf.zeroize();
    }

    // Populate effective folders by scanning message_folder_memberships once without fan-out.
    {
        let mut stmt = conn
            .prepare("SELECT message_id, account_id, folder FROM message_folder_memberships")
            .map_err(|e| StorageError::Query(e.to_string()))?;
        let mut rows = stmt
            .query([])
            .map_err(|e| StorageError::Query(e.to_string()))?;
        while let Some(row) = rows
            .next()
            .map_err(|e| StorageError::Query(e.to_string()))?
        {
            let msg_id: String = row.get(0).map_err(|e| StorageError::Query(e.to_string()))?;
            let Some(&slot) = index.id_to_slot.get(&msg_id) else {
                continue;
            };
            let mem_acc: String = row.get(1).map_err(|e| StorageError::Query(e.to_string()))?;
            if index.account_map.get(&mem_acc).copied() != Some(index.account_idx[slot]) {
                continue;
            }
            if let Some(ovr_idx) = pending_overrides[slot] {
                index.add_folder_to_slot(slot, ovr_idx);
            } else {
                let folder: String = row.get(2).map_err(|e| StorageError::Query(e.to_string()))?;
                let f_idx = index.intern_folder(&folder);
                index.add_folder_to_slot(slot, f_idx);
            }
        }
    }

    Ok(index)
}

/// Refreshes a single message slot in `index` from committed SQLite state (if embedded and valid).
pub(crate) fn refresh_slot_from_conn(
    index: &mut RamVectorIndex,
    conn: &Connection,
    message_id: &str,
) -> Result<(), StorageError> {
    let sql = format!(
        "SELECT
            m.rowid,
            m.account_id,
            v.embedding_int8,
            m.is_starred,
            m.snooze_until_epoch_ms,
            o.folder,
            {} AS effective_category,
            CASE
                WHEN p.enabled = 1 AND p.initializing = 0 THEN c.state
                ELSE NULL
            END AS review_state,
            CASE
                WHEN p.enabled = 1 AND p.initializing = 0 AND p.hide_in_inbox = 1
                     AND c.state IN ('Pending', 'Declined') THEN 1
                ELSE 0
            END AS inbox_hidden
         FROM vec_messages v
         JOIN messages m ON m.id = v.message_id
         LEFT JOIN message_folder_overrides o ON o.message_id = m.id
         LEFT JOIN sender_review_messages r ON r.message_id = m.id
         LEFT JOIN sender_review_choices c ON c.account_id = r.account_id AND c.sender = r.sender
         LEFT JOIN sender_review_preferences p ON p.account_id = r.account_id
         WHERE m.id = ?1",
        crate::search::CATEGORY_EFFECTIVE
    );
    let row = conn
        .prepare_cached(&sql)
        .map_err(|e| StorageError::Query(e.to_string()))?
        .query_row(params![message_id], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Vec<u8>>(2)?,
                r.get::<_, i64>(3)? != 0,
                r.get::<_, Option<i64>>(4)?,
                r.get::<_, Option<String>>(5)?,
                r.get::<_, Option<String>>(6)?,
                r.get::<_, Option<String>>(7)?,
                r.get::<_, i64>(8)? != 0,
            ))
        })
        .optional()
        .map_err(|e| StorageError::Query(e.to_string()))?;

    let Some((
        rowid,
        account_id,
        mut blob,
        is_starred,
        snooze_until,
        override_folder,
        effective_cat,
        review_str,
        inbox_hidden,
    )) = row
    else {
        index.remove_slot(message_id);
        return Ok(());
    };

    if blob.len() != RAM_VECTOR_DIM {
        blob.zeroize();
        return Err(StorageError::Query(
            "corrupted vec_messages dimension on incremental refresh".into(),
        ));
    }
    let mut vec_buf = [0i8; RAM_VECTOR_DIM];
    for (dst, &src) in vec_buf.iter_mut().zip(blob.iter()) {
        *dst = src as i8;
    }
    blob.zeroize();

    let mut effective_folders = Vec::new();
    let mut mem_stmt = conn
        .prepare_cached(
            "SELECT DISTINCT folder FROM message_folder_memberships WHERE message_id = ?1 AND account_id = ?2",
        )
        .map_err(|e| StorageError::Query(e.to_string()))?;
    let folders: Vec<String> = mem_stmt
        .query_map(params![message_id, account_id], |r| r.get(0))
        .map_err(|e| StorageError::Query(e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| StorageError::Query(e.to_string()))?;
    if !folders.is_empty() {
        if let Some(ovr) = override_folder {
            effective_folders.push(ovr);
        } else {
            effective_folders = folders;
        }
    }

    index.upsert_slot(
        rowid,
        message_id,
        &account_id,
        &vec_buf,
        is_starred,
        snooze_until,
        effective_cat.as_deref(),
        review_str.as_deref().and_then(parse_review_state),
        inbox_hidden,
        &effective_folders,
    )?;
    vec_buf.zeroize();
    Ok(())
}

pub(crate) fn refresh_review_for_account(
    index: &mut RamVectorIndex,
    conn: &Connection,
    account_id: &str,
) -> Result<(), StorageError> {
    let active: bool = conn
        .prepare_cached(
            "SELECT EXISTS(SELECT 1 FROM sender_review_preferences WHERE account_id=?1 AND enabled=1 AND initializing=0)",
        )
        .map_err(|e| StorageError::Query(e.to_string()))?
        .query_row([account_id], |r| r.get(0))
        .map_err(|e| StorageError::Query(e.to_string()))?;
    index.set_review_for_account_inactive(account_id);
    if !active {
        return Ok(());
    }
    let mut stmt = conn
        .prepare_cached(
            "SELECT r.message_id, c.state, p.hide_in_inbox = 1 AND c.state IN ('Pending', 'Declined')
             FROM sender_review_messages r
             JOIN sender_review_choices c ON c.account_id = r.account_id AND c.sender = r.sender
             JOIN sender_review_preferences p ON p.account_id = r.account_id
             WHERE r.account_id = ?1 AND p.enabled = 1 AND p.initializing = 0",
        )
        .map_err(|e| StorageError::Query(e.to_string()))?;
    let rows = stmt
        .query_map([account_id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)? != 0,
            ))
        })
        .map_err(|e| StorageError::Query(e.to_string()))?;
    for row in rows {
        let (msg_id, state_str, hidden) = row.map_err(|e| StorageError::Query(e.to_string()))?;
        if index.contains_id(&msg_id) {
            index.set_review_state_if_present(&msg_id, parse_review_state(&state_str), hidden);
        }
    }
    Ok(())
}
