use std::convert::AsRef;
use std::borrow::Cow;

use crate::node::Node;
use crate::query::{longest_prefix, all_keys};
use crate::iter::{LabelsIter, ValuesIter, ValuesIterMut, IntoIter, LeafPairsIter, LeafPairsIterMut};

#[derive(Debug)]
pub struct Trie<K, V> {
    size: usize,
    root: Option<Node<K, V>>,
}

impl<K, V> Trie<K, V>
{
    pub fn new() -> Self {
        Trie { size: 0, root: None }
    }

    // Retrieves key's stored value
    // (key is not in fact stored only its fragments)
    pub fn search(&self, token: K) -> Option<&'_ V>
    where K: AsRef<[u8]> 
    {
        self.root.as_ref().and_then(|n| n.search(token.as_ref()))
    }

    // Inserts value into Trie along with a key fragment if not already resident
    pub fn insert<T>(&mut self, token: T, value: V) -> Option<V>
    where T: AsRef<[u8]>
    {
        if self.root.is_none() {
            self.root = Some(Node::default());
        }

        let token_cow: Cow<[u8]> = token.as_ref().into();
        let result = self.root.as_mut().and_then(|n| n.insert(token_cow, value));

        if result.is_none() {
            self.size += 1
        }

        result
    }

    // Returns iterator of longest prefix of token that exists in trie
    pub fn longest_prefix(&self, token: K) -> Option<impl Iterator<Item = &'_ u8>>
    where K: AsRef<[u8]>   //Option<String> {
    {
        self.root.as_ref().and_then(|n| longest_prefix(n, token.as_ref()))
    }

    // Returns all keys which share a common token prefix
    pub fn all_keys(&self, token: K) -> Option<Vec<Vec<u8>>>
    where K: AsRef<[u8]>
    {
        self.root.as_ref().and_then(|n| all_keys(n, token.as_ref()))
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    pub fn clear(&mut self) {
        self.root = None;
        self.size = 0
    }

    // Removes token value and leftover key fragments as necessary
    // to the extent of possibly pruning or merging nodes
    pub fn remove(&mut self, token: K) -> Option<V>
    where K: AsRef<[u8]>
    {
        let result = self.root.as_mut().and_then(|n| n.remove(token.as_ref()));

        if result.is_some() {
            self.size -= 1
        }

        result
    }

    #[allow(dead_code)]
    pub(crate) fn root(&self) -> Option<&Node<K, V>> {
        self.root.as_ref()
    }
}

impl<K, V> Default for Trie<K, V> {
    fn default() -> Trie<K, V> {
        Trie::new()
    }
}

// Define iterator methods

impl<K, V> Trie<K, V> {
    // General reference iterator over all elements in the trie
    pub fn iter(&self) -> LeafPairsIter<'_, K, V> {
        self.root.as_ref().map_or_else(
            LeafPairsIter::default, |r| r.iter(self.size)
        )
    }

    // General mut reference iterator over all elements in the trie
    pub fn iter_mut(&mut self) -> LeafPairsIterMut<'_, K, V> {
        self.root.as_mut().map_or_else(
            LeafPairsIterMut::default, |r| r.iter_mut(self.size)
        )
    }

    // Iterate through trie's labels
    pub fn labels(&self) -> LabelsIter<'_, K, V> {
        self.root.as_ref().map_or_else(
            LabelsIter::default, |r| r.labels(self.size)
        )
    }

    // Iterate through all trie's values
    pub fn values(&self) -> ValuesIter<'_, K, V> {
        self.root.as_ref().map_or_else(
            ValuesIter::default, |r| r.values(self.size)
        )
    }

    // Iterate through all trie's values mutably
    pub fn values_mut(&mut self) -> ValuesIterMut<'_, K, V> {
        self.root.as_mut().map_or_else(
            ValuesIterMut::default, |r| r.values_mut(self.size)
        )
    }
}

impl<K, V> IntoIterator for Trie<K, V> {
    type IntoIter = IntoIter<K, V>;
    type Item = V;

    fn into_iter(self) -> Self::IntoIter {
        self.root.map_or_else(
            IntoIter::default, Node::into_iter
        )
    }
}

impl<K, V> FromIterator<(K, V)> for Trie<K, V>
where K: AsRef<[u8]>
{
    fn from_iter<I>(iter: I) -> Trie<K, V>
    where
        I: IntoIterator<Item = (K, V)>,
    {
        let mut trie = Trie::new();

        for (key, val) in iter {
            trie.insert(key, val);
        }

        trie
    }
}

// Trie unit tests

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeSet};

    fn keys_helper<'a>(keys: Option<&'a Vec<Vec<u8>>>) -> Vec<&'a str> {
        if let Some(k) = keys {
            let mut v = k.iter().map(|bytes| std::str::from_utf8(bytes).unwrap()).collect::<Vec<_>>();
            v.sort_unstable();
            v
        } else {
            vec![]
        }
    }

    fn labels_helper<'a, K: 'a, V: 'a>(labels: LabelsIter<'a, K, V>) -> BTreeSet<&'a str> {
        labels.map(|bytes| std::str::from_utf8(bytes).unwrap()).collect::<BTreeSet<&str>>()
    }

/*
    fn print_labels<'a, K: 'a, V: 'a>(labels: Labels<'a, K, V>) {
        println!("labels are {:?}", labels_helper(labels))
    }
*/

    #[test]
    fn search_basic() {
        let trie: Trie<_, _> = [("anthem", 1), ("anti", 2), ("anthemion", 7), ("and", 77)].iter().cloned().collect();
        assert_eq!(&1, trie.search("anthem").unwrap());
        assert_eq!(None, trie.search("ant"))
    }

    #[test]
    fn search_with_remove() {
        let mut trie: Trie<_, _> = [("anthem", "one"), ("anti", "two"), ("anthemion", "seven"), ("and", "seventy-seven")].iter().cloned().collect();
        assert_eq!(&"two", trie.search("anti").unwrap());
        assert_eq!(Some("two"), trie.remove("anti"));
        assert_eq!(None, trie.search("anti"));
    }

    #[test]
    fn search_with_replace_insert() {
        let mut trie: Trie<_, _> = [("anthem", 1), ("anti", 2), ("anthemion", 7), ("and", 77)].iter().cloned().collect();
        assert_eq!(&1, trie.search("anthem").unwrap());
        assert_eq!(Some(1), trie.insert("anthem", 98));
        assert_eq!(&98, trie.search("anthem").unwrap());
    }

    #[test]
    fn search_with_replace_insert_w_reference_values() {
        let important: u16 = 42;
        let i = &important;
        let vip: u16 = 98;

        let mut trie: Trie<_, _> = [("anthem", i), ("anti", i), ("anthemion", i), ("and", i)].iter().cloned().collect();
        assert_eq!(&i, trie.search("anthem").unwrap());
        assert_eq!(Some(i), trie.insert("anthem", &vip));
        assert_eq!(&&vip, trie.search("anthem").unwrap());
    }

    #[test]
    fn check_all_keys() {
        let trie: Trie<_, _> = [("anthem", 1), ("anti", 2), ("anthemion", 7), ("and", 77)].iter().cloned().collect();

        let mut keys = trie.all_keys("ant").unwrap();
        keys.sort();

        let nested = vec![
            vec![97, 110, 116, 104, 101, 109],
            vec![97, 110, 116, 104, 101, 109, 105, 111, 110],
            vec![97, 110, 116, 105]
        ];

        let flattened: Vec<u8> = nested.iter().flat_map(|v| v.iter()).cloned().collect();
        let keys_flattened: Vec<u8> = keys.iter().flat_map(|v| v.iter()).cloned().collect();

        assert_eq!(flattened, keys_flattened);

        let result = keys_helper(Some(keys.as_ref()));

        assert_eq!(vec!["anthem", "anthemion", "anti"], result);
    }

    #[test]
    fn check_longest_prefix() {
        let trie: Trie<_, _> = [("anthem", 1), ("anti", 2), ("anthemion", 7), ("and", 77)].iter().cloned().collect();

        let result = trie.longest_prefix("anthemio").unwrap().cloned().collect::<Vec<_>>();

        assert_eq!("anthem", std::str::from_utf8(&result).unwrap());
    }


    #[test]
    fn passthru_removes() {
        let mut trie: Trie<_, _> = [("anthem", 1), ("anti", 2), ("anthemion", 7), ("and", 77)].iter().cloned().collect();

        let keys = trie.all_keys("an");
        let result = keys_helper(keys.as_ref());

        assert_eq!(vec!["and", "anthem", "anthemion", "anti"], result);


        // remove passthru that is followed by a pruned edge
        assert_eq!(2, trie.remove("anti").unwrap());
        assert_eq!(None, trie.search("anti"));
        assert_eq!(&1, trie.search("anthem").unwrap());

        // remove passthrough that has a child
        assert_eq!(1, trie.remove("anthem").unwrap());

        let keys = trie.all_keys("an");
        let result = keys_helper(keys.as_ref());

        assert_eq!(vec!["and", "anthemion"], result);
    }


    #[test]
    fn delete_all() {
        let mut trie: Trie<_, _> = [("anthem", 1), ("anti", 2), ("anthemion", 7), ("and", 77)].iter().cloned().collect();

        let keys = trie.all_keys("an");
        let result = keys_helper(keys.as_ref());

        assert_eq!(vec!["and", "anthem", "anthemion", "anti"], result);

//        print_labels(trie.labels());

        // skip the first &str "and" then delete it after the loop 
        for (i, k) in result.iter().skip(1).enumerate() {
            trie.remove(k);

//            print_labels(trie.labels());

            let keys = trie.all_keys("an");
            let v: Vec<&str> = keys_helper(keys.as_ref());

            match i {
                0 => assert_eq!(v, vec!["and", "anthemion", "anti"]),
                1 => assert_eq!(v, vec!["and", "anti"]),
                2 => assert_eq!(v, vec!["and"]),
                _ => (),
            }

            assert_eq!(trie.remove("nonexistent1"), None);
        }

        assert_eq!(trie.remove("and").unwrap(), 77);
        assert_eq!(trie.all_keys("an"), None);
        assert_eq!(trie.remove("nonexistent2"), None);
        assert_eq!(trie.is_empty(), true);
    }


    #[test]
    fn check_compessed_labels() {
        let mut trie: Trie<_, _> = [("anthem", 1), ("anti", 2), ("anthemion", 7), ("and", 77)].iter().cloned().collect();

        let keys = trie.all_keys("an");
        let keys_vec = keys_helper(keys.as_ref());

        assert_eq!(vec!["and", "anthem", "anthemion", "anti"], keys_vec);

        // skip the first &str "and" then delete it after the loop 
        for (i, k) in keys_vec.iter().enumerate() {

            let set = labels_helper(trie.labels());

            // Through each delete iteration, check the labels of the trees
            // Since the deletion causes labels to be compressed/deleted expect number of labels to reduce
            match i {
                0 => assert_eq!(set, BTreeSet::from(["an", "d", "hem", "i", "ion", "t"])),
                1 => assert_eq!(set, BTreeSet::from(["ant", "hem", "i", "ion"])),
                2 => assert_eq!(set, BTreeSet::from(["ant", "hemion", "i"])),
                3 => assert_eq!(set, BTreeSet::from(["anti"])),
                _ => (),
            }

            trie.remove(k);
        }
    }


    #[test]
    fn check_values_iter() {
        let mut trie: Trie<_, _> = [("anthem", 1), ("anti", 2), ("anthemion", 7), ("and", 77)].iter().cloned().collect();

        let _ = trie.values_mut().map(|v| { *v = *v * 5; v } ).collect::<BTreeSet<&mut i32>>();
        assert_eq!(5, trie.remove("anthem").unwrap());

        let set2 = trie.values().collect::<BTreeSet<&i32>>();
        assert_eq!(BTreeSet::from([&10, &35, &385]), set2)
    }

    #[test]
    fn check_values_into_iter() {
        let trie: Trie<_, _> = [("anthem", 1), ("anti", 2), ("anthemion", 7), ("and", 77)].iter().cloned().collect();
        let vec1 = trie.into_iter().map(|mut v| { v = v + 1; v } ).collect::<BTreeSet<i32>>();
        assert_eq!(vec1, BTreeSet::from([2, 3, 8, 78]));
    }

    #[test]
    fn check_leafpairs_iter() {
        let trie: Trie<_, _> = [("anthem", 1), ("anti", 2), ("anthemion", 7), ("and", 77)].iter().cloned().collect();
        let set = trie.iter().collect::<BTreeSet<(&[u8], &i32)>>();
        assert_eq!(BTreeSet::from([("d".as_bytes(), &77), ("hem".as_bytes(), &1), ("i".as_bytes(), &2), ("ion".as_bytes(), &7)]), set)
    }
}

