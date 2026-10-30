use super::PropSource;

pub(crate) struct List<'a>(pub(crate) &'a [(u32, &'a str)]);

impl PropSource for List<'_> {
    fn prop(&self, key: &str) -> Option<&str> {
        self.prop_hash(crate::flash::fnv_lower(key))
    }

    fn prop_hash(&self, hash: u32) -> Option<&str> {
        self.0.iter().rev().find(|p| p.0 == hash).map(|p| p.1)
    }

    fn prop_hash_all(&self, hash: u32) -> Vec<&str> {
        self.0.iter().filter(|p| p.0 == hash).map(|p| p.1).collect()
    }
}
