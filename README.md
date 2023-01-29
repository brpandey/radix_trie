Radix Trie
==========

A simple space-optimized trie written in Rust

```rust
use radix_trie::trie::Trie;

fn main() {
    // Radix Trie Example
    println!("Search suggestions");

    // Search terms is a trie,
    // K is AsRef<[u8]> which &str is and u16 fits into any generic V
    let search_terms: Trie<&str, u16> =
        [("mobile", 10),("mandala", 67),("mousy brown hair dye", 23),("moneypot", 45),
         ("mexican sombrero", 27), ("muscle cars", 11), ("mouthguard", 8),
         ("monitor", 7),("mousepad", 2361), ("muave eraser", 98)]
        .iter().cloned().collect();

    let mut keys;
    let searched_word = "mouse".bytes();
    let mut user_typed = vec![];

    // Simulate user typing in a search query and
    // viewing the dropdown box of potentially matching results
    // as user is typing in the searched_word
    for (i, b) in searched_word.enumerate() {
        user_typed.push(b); // add each character (byte in this case) to user_typed

        // retrieve list of matching keys from trie
        keys = search_terms.all_keys(std::str::from_utf8(&user_typed).unwrap_or_default());

        let v: Vec<&str> = flatten_keys(keys.as_ref());
        println!("Search results, for typed text: {:?} ---> {:?}\n",
                 std::str::from_utf8(&user_typed).unwrap_or_default(),
                 &v);

        match i {
            0 => assert_eq!(v, vec!["mandala", "mexican sombrero", "mobile", "moneypot",
                                    "monitor", "mousepad", "mousy brown hair dye", 
                                    "mouthguard", "muave eraser", "muscle cars"]),
            1 => assert_eq!(v, vec!["mobile", "moneypot", "monitor", "mousepad", 
            "mousy brown hair dye", "mouthguard"]),
            2 => assert_eq!(v, vec!["mousepad", "mousy brown hair dye", "mouthguard"]),
            3 => assert_eq!(v, vec!["mousepad", "mousy brown hair dye"]),
            4 => assert_eq!(v, vec!["mousepad"]),
            _ => (),
        }
    }
}

pub fn flatten_keys<'a>(keys: Option<&'a Vec<Vec<u8>>>) -> Vec<&'a str> {
    keys.map_or(vec![], |k| {
        let mut v = k.iter()
            .map(|bytes| std::str::from_utf8(bytes).unwrap_or_default())
            .collect::<Vec<_>>();
        v.sort_unstable();
        v
    })
}
```

```rust
Search suggestions

Search results, for typed text: "m" ---> 
["mandala", "mexican sombrero", "mobile", "moneypot", "monitor", "mousepad", 
"mousy brown hair dye", "mouthguard", "muave eraser", "muscle cars"]

Search results, for typed text: "mo" ---> 
["mobile", "moneypot", "monitor", "mousepad", "mousy brown hair dye", "mouthguard"]

Search results, for typed text: "mou" ---> 
["mousepad", "mousy brown hair dye", "mouthguard"]

Search results, for typed text: "mous" ---> 
["mousepad", "mousy brown hair dye"]

Search results, for typed text: "mouse" ---> 
["mousepad"]
```

Upon deletion handles a combination of unmarking, pruning, and compression
Picture taken from Advanced Algorithms and Data Structures by Marcello La Rocca 

<p float="left">
  <img src='images/insert.png' width='845' height='450'/> 
</p>


Thanks!
Bibek


PS (with love)

Some tests ;)

```rust

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

        // skip the first &str "and" then delete it after the loop 
        for (i, k) in result.iter().skip(1).enumerate() {
            trie.remove(k);

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
    fn check_compressed_labels() {
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

```
