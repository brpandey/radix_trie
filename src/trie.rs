use crate::node::Node;
use crate::query::{longest_prefix, all_keys};
use crate::iter::LabelsIter;

#[derive(Debug)]
pub struct Trie {
    size: usize,
    root: Option<Node>,
}

impl Trie {
    pub fn new() -> Self {
        Trie { size: 0, root: None }
    }

    pub fn search(&self, token: &str) -> Option<&'_ i32> {
        self.root.as_ref().and_then(|n| n.search(token))
    }

    pub fn insert(&mut self, token: &str, value: i32) -> Option<i32> {
        if self.root.is_none() {
            self.root = Some(Node::default());
        }

        let result = self.root.as_mut().and_then(|n| n.insert(token, Some(value)));

        if result.is_none() {
            self.size += 1
        }

        result
    }

    pub fn longest_prefix(&self, token: &str) -> Option<impl Iterator<Item = &'_ u8>> { //Option<String> {
        self.root.as_ref().and_then(|n| longest_prefix(n, token))
    }

    pub fn all_keys(&self, token: &str) -> Option<Vec<Vec<u8>>> {
        self.root.as_ref().and_then(|n| all_keys(n, token))
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    pub fn clear(&mut self) {
        self.root = None;
        self.size = 0
    }

    pub fn remove(&mut self, token: &str) -> Option<i32> {
        let result = self.root.as_mut().and_then(|n| n.remove(token));

        if result.is_some() {
            self.size -= 1
        }

        result
    }
}


impl Default for Trie {
    fn default() -> Self {
        Self::new()
    }
}


// top level iterator for Trie
pub struct Labels<'a> {
    inner: LabelsIter<'a>,
    size: usize,
}

impl Trie {
    pub fn iter(&self) -> Labels<'_> {
        Labels {
            inner: self.root.as_ref().map_or_else(
                || LabelsIter::default(), |r| r.iter()
            ),
            size: self.size,
        }
    }
}


impl<'a> Iterator for Labels<'a> {
    type Item = &'a [u8];

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.size, Some(self.size))
    }

}


impl FromIterator<(&'static str, i32)> for Trie {
    fn from_iter<I>(iter: I) -> Trie
    where
        I: IntoIterator<Item = (&'static str, i32)>,
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

    fn labels_helper<'a>(labels: Labels<'a>) -> BTreeSet<&'a str> {
        labels.map(|bytes| std::str::from_utf8(bytes).unwrap()).collect::<BTreeSet<&str>>()
    }

    /*
    fn print_labels<'a>(labels: Labels<'a>) {
        println!("labels are {:?}", labels_helper(labels))
    }
    */

    #[test]
    fn search_basic() {
        let t: Trie = [("anthem", 1), ("anti", 2), ("anthemion", 7), ("and", 77)].iter().cloned().collect();
        assert_eq!(&1, t.search("anthem").unwrap());
        assert_eq!(None, t.search("ant"))
    }

    #[test]
    fn search_with_remove() {
        let mut t: Trie = [("anthem", 1), ("anti", 2), ("anthemion", 7), ("and", 77)].iter().cloned().collect();
        assert_eq!(&2, t.search("anti").unwrap());
        assert_eq!(Some(2), t.remove("anti"));
        assert_eq!(None, t.search("anti"));
    }

    #[test]
    fn search_with_replace_insert() {
        let mut t: Trie = [("anthem", 1), ("anti", 2), ("anthemion", 7), ("and", 77)].iter().cloned().collect();
        assert_eq!(&1, t.search("anthem").unwrap());
        assert_eq!(Some(1), t.insert("anthem", 98));
        assert_eq!(&98, t.search("anthem").unwrap());
    }

    #[test]
    fn check_all_keys() {
        let t: Trie = [("anthem", 1), ("anti", 2), ("anthemion", 7), ("and", 77)].iter().cloned().collect();

        let mut keys = t.all_keys("ant").unwrap();
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
        let t: Trie = [("anthem", 1), ("anti", 2), ("anthemion", 7), ("and", 77)].iter().cloned().collect();

        let result = t.longest_prefix("anthemio").unwrap().cloned().collect::<Vec<_>>();

        assert_eq!("anthem", std::str::from_utf8(&result).unwrap());
    }

     

    #[test]
    fn passthru_removes() {
        let mut t: Trie = [("anthem", 1), ("anti", 2), ("anthemion", 7), ("and", 77)].iter().cloned().collect();

        let keys = t.all_keys("an");
        let result = keys_helper(keys.as_ref());

        assert_eq!(vec!["and", "anthem", "anthemion", "anti"], result);


        // remove passthru that is followed by a pruned edge
        assert_eq!(2, t.remove("anti").unwrap());
        assert_eq!(None, t.search("anti"));
        assert_eq!(&1, t.search("anthem").unwrap());

        // remove passthrough that has a child
        assert_eq!(1, t.remove("anthem").unwrap());

        let keys = t.all_keys("an");
        let result = keys_helper(keys.as_ref());

        assert_eq!(vec!["and", "anthemion"], result);
    }


    #[test]
    fn delete_all() {
        let mut t: Trie = [("anthem", 1), ("anti", 2), ("anthemion", 7), ("and", 77)].iter().cloned().collect();

        let keys = t.all_keys("an");
        let result = keys_helper(keys.as_ref());

        assert_eq!(vec!["and", "anthem", "anthemion", "anti"], result);

//        print_labels(t.iter());

        // skip the first &str "and" then delete it after the loop 
        for (i, k) in result.iter().skip(1).enumerate() {
            t.remove(k);

//            print_labels(t.iter());

            let keys = t.all_keys("an");
            let v: Vec<&str> = keys_helper(keys.as_ref());

            match i {
                0 => assert_eq!(v, vec!["and", "anthemion", "anti"]),
                1 => assert_eq!(v, vec!["and", "anti"]),
                2 => assert_eq!(v, vec!["and"]),
                _ => (),
            }

            assert_eq!(t.remove("nonexistent1"), None);
        }

        assert_eq!(t.remove("and").unwrap(), 77);
        assert_eq!(t.all_keys("an"), None);


        assert_eq!(t.remove("nonexistent2"), None);

        assert_eq!(t.is_empty(), true);
    }


    #[test]
    fn check_compessed_labels() {
        let mut t: Trie = [("anthem", 1), ("anti", 2), ("anthemion", 7), ("and", 77)].iter().cloned().collect();

        let keys = t.all_keys("an");
        let keys_vec = keys_helper(keys.as_ref());

        assert_eq!(vec!["and", "anthem", "anthemion", "anti"], keys_vec);

        // skip the first &str "and" then delete it after the loop 
        for (i, k) in keys_vec.iter().enumerate() {

            let set = labels_helper(t.iter());

            // Through each delete iteration, check the labels of the trees
            // Since the deletion causes labels to be compressed/deleted expect number of labels to reduce
            match i {
                0 => assert_eq!(set, BTreeSet::from(["an", "d", "hem", "i", "ion", "t"])),
                1 => assert_eq!(set, BTreeSet::from(["ant", "hem", "i", "ion"])),
                2 => assert_eq!(set, BTreeSet::from(["ant", "hemion", "i"])),
                3 => assert_eq!(set, BTreeSet::from(["anti"])),
                _ => (),
            }

            t.remove(k);
        }
    }
    
}

