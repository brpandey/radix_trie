use radix_trie::trie::{Trie, LabelsIter};
use std::collections::BTreeSet;

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

    let set = labels_helper(search_terms.labels());

    // Check the label of the static tree (see picture diagram)
    assert_eq!(set, BTreeSet::from(["andala", "ave eraser", "bile", "epad", "exican sombrero", "eypot",
                                    "itor", "m", "n", "o", "s", "scle cars", "thguard", "u", "y brown hair dye"]));

    let ks = search_terms.all_keys("me");
    assert_eq!(vec!["mexican sombrero"], flatten_keys(ks.as_ref()));

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
        println!("Search results, for typed text: {:?} ---> {:?}",
                 std::str::from_utf8(&user_typed).unwrap_or_default(),
                 &v);

        match i {
            0 => assert_eq!(v, vec!["mandala", "mexican sombrero", "mobile", "moneypot",
                                    "monitor", "mousepad", "mousy brown hair dye",
                                    "mouthguard", "muave eraser", "muscle cars"]),
            1 => assert_eq!(v, vec!["mobile", "moneypot", "monitor", "mousepad",
                                    "mousy brown hair dye", "mouthguard"]),
            2 => assert_eq!(v, vec!["mousepad", "mousy brown hair dye",
                                    "mouthguard"]),
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

fn labels_helper<'a, K: 'a, V: 'a>(labels: LabelsIter<'a, K, V>) -> BTreeSet<&'a str> {
    labels.map(|bytes| std::str::from_utf8(bytes).unwrap()).collect::<BTreeSet<&str>>()
}



/*
Search suggestions
Search results, for slowly typed text: "m" ---> ["mandala", "mexican sombrero", "mobile", "moneypot", "monitor", "mousepad", "mousy brown hair dye", "mouthguard", "muave eraser", "muscle cars"]
Search results, for slowly typed text: "mo" ---> ["mobile", "moneypot", "monitor", "mousepad", "mousy brown hair dye", "mouthguard"]
Search results, for slowly typed text: "mou" ---> ["mousepad", "mousy brown hair dye", "mouthguard"]
Search results, for slowly typed text: "mous" ---> ["mousepad", "mousy brown hair dye"]
Search results, for slowly typed text: "mouse" ---> ["mousepad"]
*/
