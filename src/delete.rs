use std::collections::HashSet;

use std::fmt::Debug;

use crate::node::{Node, EdgeType};
use crate::traverse::{traverse, TraverseItem, TraverseType, TraverseResult};
use crate::macros::enum_extract;

type DeletePlan = Vec<Playback>;

// Specifies node level and edge_key when traversing along a node path
// of a given prefix
// e.g. Link(3, 104) denotes a node at level 3 with edge key 104
// (root being level 0)
#[derive(Debug, PartialEq)]
pub enum Cursor {
    Node(u32),
    Link(u32, u8),
    DoubleLink(u32, u8, u8),
}

// Tags the node operation type given a prefix's node path
// e.g. Unmark means we set it up for deletion by removing its key status
#[derive(Debug, PartialEq)]
pub enum Playback {
    Unmark(Cursor),
    Prune(Cursor),
    MergeTemp(u8),
    Merge(Cursor),
    Keep(Cursor),
}

// State to track what operations have been performed
// while a delete plan is being created
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum Status {
    Deleted,
    DeletedPruned,
    Merged,
}

// Internal action states used during
// delete plan formation
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum Action {
    Prune,
    Merge,
    Noop,
}

// In order to delete a node without using back or parent links we create a replay stack which
// gives us the required "plan" "info that only uses copy semantics to aid in eventual
// deleting a node or pruning a node's edge with a single mutable ref

// (Rust supports recursion yet not tail recursion - the explicit stack is on the heap
// so it avoids  concerns of potentially blowing the call stack for long sequences)
pub fn capture<K, V>(current: &Node<K, V>, prefix: &[u8]) -> Option<DeletePlan> {
    let mut replay: Vec<Playback> = Vec::new();
    let mut status: HashSet<Status> = HashSet::new();
    let mut action: Action = Action::Noop;

    // Essentially reduce the multiple node immutable refs stack into
    // a replay stack which just has copy values / semantics

    // Take the dfs stack (with the terminal node on top)
    // and convert it into a replay stack

    let result: TraverseResult<K, V> =  traverse(current, prefix, TraverseType::Fold)?;
    let mut stack = enum_extract!(result, TraverseResult::Stack);

    //prepopulated stack given prefix and trie
    if let Some(TraverseItem{node, next_key: _, label: _, level}) = stack.pop() {
        if node.is_key() {
            replay.push(Playback::Unmark(Cursor::Node(level)));

            status.insert(Status::Deleted);

            // If no child edges then can easily prune, otherwise if single we have a passthrough
            match node.edge_type() {
                None => action = Action::Prune,
                Some(EdgeType::Single) => {
                    // store key (temporarily) that will be used as the merge key / merge node
                    // when we merge the passthrough node's label with the merge node
                    let merge_key = node.edges_keys_iter().copied().collect::<Vec<u8>>().pop().unwrap();

                    let item = Playback::MergeTemp(merge_key);
                    replay.push(item);

                    action = Action::Merge
                },
                _ => (),
            }
        }
    }

    // Work backwards from the node we want to delete
    while !stack.is_empty() {
        let TraverseItem{node, next_key, label: _, level} = stack.pop().unwrap();

        match action {
            Action::Prune => {
                // We can only prune a level above the node that needs deleting
                let info = Cursor::Link(level, next_key);
                let item = Playback::Prune(info);
                replay.push(item);

                // Prune once since when we insert, everything is already compressed,
                // only have to prune the outgoing edge of parent to the node to delete

                status.remove(&Status::Deleted);
                status.insert(Status::DeletedPruned);
            },
            Action::Merge => {
                match replay.pop() {
                    // Form double link cursor used with eventual merge operation
                    // if level marks node x, next_key refers to child node x''
                    // and merge_key refers to grand child node x'''
                    Some(Playback::MergeTemp(merge_key)) => {
                        let info = Cursor::DoubleLink(level, next_key, merge_key);
                        let item = Playback::Merge(info);

                        replay.push(item);
                        status.insert(Status::Merged);
                    },
                    _ => unreachable!()
                }
            },
            Action::Noop => {
                replay.push(Playback::Keep(Cursor::Link(level, next_key)));
            },
        }

        // A  passthrough node is able to be compressed only after a single prune
        if action == Action::Prune &&
            status.contains(&Status::DeletedPruned) && status.len() == 1 &&
            !node.is_key() && node.edge_type().unwrap() == EdgeType::Branching(2) {

                // record key that will be used as the merge key / merge node
                // when we merge the passthrough node's label with the merge node
                let mut set = node.edges_keys_iter().collect::<HashSet<_>>();
                set.remove(&next_key);
                let merge_key = set.into_iter().copied().collect::<Vec<u8>>().pop().unwrap();

                let item = Playback::MergeTemp(merge_key);
                replay.push(item);

                action = Action::Merge
            } else {
                action = Action::Noop
            }
    }

    // If replay is empty (or predicate is false) return None otherwise Some
    Some(replay).filter(|r| !r.is_empty())

}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::trie::Trie;

    use Playback as P;
    use Cursor as C;

    // Verify the delete plan that is generated for these prefix tokens is accurate
    #[test]
    fn check_delete_plan() {
        let mut trie: Trie<_, _> = [("anthem", 1), ("anti", 2), ("anthemion", 7), ("and", 77)].iter().rev().cloned().collect();

        // skip the first &str "and" then delete it after the loop
        let result = vec!["anthemion", "anthem", "and", "anti"];

        let mut i = 0;

        let root = trie.root();
        let pb = capture(root.unwrap(), result[i].as_bytes()).unwrap();

        assert_eq!(pb, vec![P::Unmark(C::Node(4)), P::Prune(C::Link(3, 105)),
                            P::Keep(C::Link(2, 104)), P::Keep(C::Link(1, 116)), P::Keep(C::Link(0, 97))]);

        trie.remove(&result[i]);
        i+=1;

        let root = trie.root();
        let pb = capture(root.unwrap(), result[i].as_bytes()).unwrap();

        assert_eq!(pb, vec![P::Unmark(C::Node(3)), P::Prune(C::Link(2, 104)), P::Merge(C::DoubleLink(1, 116, 105)), P::Keep(C::Link(0, 97))]);

        trie.remove(&result[i]);
        i+=1;

        let root = trie.root();
        let pb = capture(root.unwrap(), result[i].as_bytes()).unwrap();

        assert_eq!(pb, vec![P::Unmark(C::Node(2)), P::Prune(C::Link(1, 100)), P::Merge(C::DoubleLink(0, 97, 116))]);

        trie.remove(&result[i]);
        i+=1;

        let root = trie.root();
        let pb = capture(root.unwrap(), "anti".as_bytes()).unwrap();

        assert_eq!(pb, vec![P::Unmark(C::Node(1)), P::Prune(C::Link(0, 97))]);

        trie.remove(&result[i]);

        assert!(trie.is_empty());
    }
}
