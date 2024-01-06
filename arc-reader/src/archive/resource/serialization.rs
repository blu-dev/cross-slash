use std::{any::TypeId, collections::HashMap};

use indexmap::IndexSet;

use crate::index::{checked_range, INVALID_INDEX};

pub(crate) struct SerState {
    type_map: HashMap<TypeId, IndexSet<u32>>,
}

impl SerState {
    pub fn new() -> Self {
        Self {
            type_map: HashMap::with_capacity(48),
        }
    }

    #[track_caller]
    pub fn get<T: 'static>(&self, index: u32) -> u32 {
        if index == INVALID_INDEX {
            return index;
        }

        let map = self.type_map.get(&TypeId::of::<T>()).unwrap_or_else(|| {
            panic!(
                "Failed to get the reserved indexes for {}",
                std::any::type_name::<T>()
            );
        });

        let index = map.get_index_of(&index).unwrap_or_else(|| {
            panic!(
                "Index {index:?} is not reserved for {}",
                std::any::type_name::<T>()
            );
        });

        index as u32
    }

    #[track_caller]
    pub fn reserve<T: 'static>(&mut self, index: u32) -> u32 {
        if index == INVALID_INDEX {
            return index;
        }

        let set = self
            .type_map
            .entry(TypeId::of::<T>())
            .or_insert_with(|| IndexSet::with_capacity(0x4000));

        let (index, did_insert) = set.insert_full(index);

        if !did_insert {
            panic!(
                "Failed to insert index {index:?} for {} because it is already reserved",
                std::any::type_name::<T>()
            );
        }

        index as u32
    }

    pub fn try_reserve<T: 'static>(&mut self, index: u32) -> bool {
        if index == INVALID_INDEX {
            return false;
        }

        let set = self
            .type_map
            .entry(TypeId::of::<T>())
            .or_insert_with(|| IndexSet::with_capacity(0x4000));

        set.insert(index)
    }

    pub fn try_get<T: 'static>(&self, index: u32) -> Option<u32> {
        if index == INVALID_INDEX {
            return None;
        }

        self.type_map
            .get(&TypeId::of::<T>())?
            .get_index_of(&index)
            .map(|idx| idx as u32)
    }

    #[track_caller]
    pub fn reserve_range<T: 'static>(&mut self, index: u32, count: u32) -> u32 {
        if index == INVALID_INDEX {
            if count != 0 {
                panic!("Range is pointing to invalid index with non-zero count");
            }
            return index;
        };

        let set = self
            .type_map
            .entry(TypeId::of::<T>())
            .or_insert_with(|| IndexSet::with_capacity(0x4000));

        let mut start_index = None;

        for index in checked_range(index, count) {
            let (index, did_insert) = set.insert_full(index);

            if !did_insert {
                panic!("Failed to insert index {index:?} as part of range for {} because it is already reserved", std::any::type_name::<T>());
            }

            if start_index.is_none() {
                start_index = Some(index as u32);
            }
        }

        start_index.unwrap_or(0)
    }

    pub fn iter<T: 'static>(&self) -> impl Iterator<Item = u32> + '_ {
        self.type_map
            .get(&TypeId::of::<T>())
            .into_iter()
            .flat_map(|set| set.iter())
            .copied()
    }
}
