use crate::checks::bin_tree_with_parent::NodeCursor;
use digest::Output;
use itertools::Itertools;
use pace26io::binary_tree::TopDownCursor;
use pace26io::newick::NewickWriter;
use serde::de::{Error, Unexpected};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};
use std::io::Write;
pub const DIGEST_HEX_DIGITS: usize = 32;
type Algo = Sha256;

/// Computes a hash digest for a binary tree.
///
/// # Warning
/// Modifies the tree by normalizing the order of each inner leaf.
fn digest_bintree(tree: NodeCursor) -> Output<Algo> {
    struct WriteAdapter(Algo);
    impl Write for WriteAdapter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.update(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let mut writer = WriteAdapter(Algo::new());
    tree.normalize_child_order();
    tree.write_newick(&mut writer).unwrap();
    writer.0.finalize()
}

fn digest_digests(digests: &mut [Output<Algo>]) -> Output<Algo> {
    digests.sort_unstable();
    let mut hasher = Algo::new();

    for d in digests {
        hasher.update(d);
    }

    hasher.finalize()
}

/// Computes the digest of an instance. The digest is invariant in the order of trees and
/// swapping of children. The first digit indicates the number approximate number of trees,
/// the second the approximate number of leaves (both in a logarithmic scale). The digest value
/// is returned in hexadecimal representation containing exactly [`DIGEST_HEX_DIGITS`] digits.
///
/// # Warning
/// Modifies the tree by normalizing the order of each inner leaf.
pub fn digest_instance(trees: Vec<NodeCursor>, num_leaves: u32) -> DigestString {
    let num_trees = trees.len() as u32;

    let digest = {
        let mut digests: Vec<_> = trees.into_iter().map(digest_bintree).collect();
        digest_digests(&mut digests)
    };

    // we use a logarithmic scale to indicate the approximate number of trees and leaves
    let tree_score = num_trees.ilog2().saturating_sub(1).min(0xf);
    let leaves_score = num_leaves.ilog(2).saturating_sub(3).min(0xf);

    let mut result = Vec::with_capacity(DIGEST_HEX_DIGITS);
    write!(&mut result, "{tree_score:x}{leaves_score:x}").unwrap();

    for x in digest.iter().copied().take(DIGEST_HEX_DIGITS / 2 - 1) {
        write!(&mut result, "{x:02x}").unwrap();
    }

    assert_eq!(result.len(), DIGEST_HEX_DIGITS);
    DigestString(String::from_utf8(result).unwrap()) // TODO: we could avoid this check, but requires unsafe
}

/// Computes the digest of an instance. The digest is invariant in the order of trees and
/// swapping of children. The first four digits indicate the number of trees in the solution
/// (clamped at 0xffff). The digest value is returned in hexadecimal representation
/// containing exactly [`DIGEST_HEX_DIGITS`] digits.
///
/// Since we assume that this function is only used for feasible solutions, there is no need
/// to include isolated nodes (i.e. a tree where the root is the sole leaf). Such trees are
/// ignored in the input, but can also be omitted. For this reason, the solution's size (including
/// isolated nodes) needs to be passed explicitly.
///
/// # Warning
/// Modifies the tree by normalizing the order of each inner leaf.
pub fn digest_solution(trees: Vec<NodeCursor>, score: u32) -> DigestString {
    let digest = {
        let mut digests: Vec<_> = trees
            .into_iter()
            .filter(|root| !root.is_leaf())
            .map(digest_bintree)
            .collect();
        digest_digests(&mut digests)
    };

    let mut result = Vec::with_capacity(DIGEST_HEX_DIGITS);
    write!(&mut result, "{:04x}", score.min(0xffff)).unwrap();

    for x in digest.iter().copied().take(DIGEST_HEX_DIGITS / 2 - 2) {
        write!(&mut result, "{x:02x}").unwrap();
    }

    assert_eq!(result.len(), DIGEST_HEX_DIGITS);
    DigestString(String::from_utf8(result).unwrap()) // TODO: we could avoid this check, but requires unsafe
}

#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct DigestString(String);

impl DigestString {
    pub fn new(str: String) -> Option<Self> {
        if str.len() != DIGEST_HEX_DIGITS {
            return None;
        }
        if str.chars().any(|c| !c.is_digit(16)) {
            return None;
        }
        Some(DigestString(str))
    }

    pub fn new_from_binary(bin: &[u8]) -> Option<Self> {
        if 2 * bin.len() != DIGEST_HEX_DIGITS {
            return None;
        }
        let mut str_data: Vec<u8> = Vec::with_capacity(DIGEST_HEX_DIGITS);
        for &x in bin {
            write!(&mut str_data, "{:02x}", x).unwrap();
        }
        Some(Self(String::from_utf8(str_data).unwrap()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn to_bin(&self) -> [u8; DIGEST_HEX_DIGITS / 2] {
        let mut result = [0u8; DIGEST_HEX_DIGITS / 2];

        for ((a, b), target) in self.0.chars().tuples().zip(result.iter_mut()) {
            *target = ((a.to_digit(16).unwrap() as u8) << 4) | (b.to_digit(16).unwrap() as u8);
        }

        result
    }

    pub fn to_boxed_bin(&self) -> Box<[u8]> {
        Box::new(self.to_bin())
    }
}

impl Serialize for DigestString {
    fn serialize<S>(&self, ser: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        ser.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for DigestString {
    fn deserialize<D>(de: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut s = String::deserialize(de)?;

        if s.len() != DIGEST_HEX_DIGITS {
            return Err(D::Error::invalid_length(
                s.len(),
                &"hex digest with exactly 20 characters",
            ));
        }
        if !s.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(D::Error::invalid_value(
                Unexpected::Str(&s),
                &"a valid hex string",
            ));
        }

        s = s.to_lowercase();

        Ok(DigestString(s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checks::bin_tree_with_parent::BinTreeWithParentBuilder;
    use hex_literal::hex;
    use pace26io::binary_tree::NodeIdx;
    use pace26io::newick::BinaryTreeParser;

    #[test]
    fn digest_bintree() {
        const TREE: &str = "((3,4),(2,1));"; // the hash digest below was computed for the string "((1,2),(3,4));"
        let tree = BinTreeWithParentBuilder::default()
            .parse_newick_from_str(TREE, NodeIdx::default())
            .unwrap();
        let digest = super::digest_bintree(tree);
        assert_eq!(
            digest[..],
            hex!("5aecb10e41777da0a300dae254d01a2fad3fd892d0b3b553821e2e684194a1f6")
        );
    }

    #[test]
    fn digest_instance() {
        // the following instances need to receive the same digest
        let instances = vec![
            vec!["((3,4),(2,1));", "(1,(2,(3,4)));"],
            vec!["((4,3),(1,2));", "(1,(2,(4,3)));"],
            vec!["(1,(2,(3,4)));", "((3,4),(2,1));"],
            vec!["(1,(2,(4,3)));", "((4,3),(1,2));"],
        ];

        let mut previous_hash = None;

        for instance in instances {
            let trees: Vec<_> = instance
                .iter()
                .map(|&nw| {
                    BinTreeWithParentBuilder::default()
                        .parse_newick_from_str(nw, NodeIdx::default())
                        .unwrap()
                })
                .collect();
            let digests = super::digest_instance(trees, 4);

            if let Some(previous_hash) = previous_hash {
                assert_eq!(digests, previous_hash);
            }

            previous_hash = Some(digests);
        }
    }

    #[test]
    fn digest_solution() {
        // the following instances need to receive the same digest
        let solutions = vec![
            vec!["((3,4),(2,1));", "(5,(6,(7,8)));", "9;"],
            vec!["((3,4),(1,2));", "9;", "(5,(6,(8,7)));"],
            vec!["((3,4),(2,1));", "(5,(6,(8,7)));"],
            vec!["((3,4),(1,2));", "(5,(6,(7,8)));"],
        ];

        let mut previous_hash = None;

        for sol in solutions {
            let trees: Vec<_> = sol
                .iter()
                .map(|&nw| {
                    BinTreeWithParentBuilder::default()
                        .parse_newick_from_str(nw, NodeIdx::default())
                        .unwrap()
                })
                .collect();
            let digests = super::digest_solution(trees, 3);

            if let Some(previous_hash) = previous_hash {
                assert_eq!(digests, previous_hash);
            }

            previous_hash = Some(digests);
        }
    }

    #[test]
    fn digest_string_serde() {
        let string = (0..DIGEST_HEX_DIGITS)
            .map(|i| format!("{:x}", i % 16))
            .collect::<String>();
        assert_eq!(string.len(), DIGEST_HEX_DIGITS);

        let digest_string = DigestString::new(string).unwrap();

        let serialized = serde_json::to_string(&digest_string).unwrap();
        let deserialized: DigestString = serde_json::from_str(&serialized).unwrap();

        assert_eq!(digest_string, deserialized);

        assert!(serde_json::from_str::<DigestString>("\"xasdf\"").is_err());
    }

    #[test]
    fn digest_deserialize_wrong_length() {
        assert!(serde_json::from_str::<DigestString>("\"0123\"").is_err());
    }

    #[test]
    fn digest_deserialize_invalid_chars() {
        assert!(
            serde_json::from_str::<DigestString>("\"01234567890123456789012345678931\"").is_ok()
        );
        assert!(
            serde_json::from_str::<DigestString>("\"0123456789012345678901234567893z\"").is_err()
        );
    }

    #[test]
    fn digest_string() {
        let mut string = (0..DIGEST_HEX_DIGITS)
            .map(|i| format!("{:x}", i % 16))
            .collect::<String>();
        assert_eq!(string.len(), DIGEST_HEX_DIGITS);

        assert_eq!(DigestString::new(string.clone()).unwrap().as_str(), &string);

        string.pop();
        assert!(DigestString::new(string.clone()).is_none());

        string.push('X'); // this is not a hex digit ;)
        assert!(DigestString::new(string.clone()).is_none());

        string.pop();
        string.push('F');
        assert_eq!(DigestString::new(string.clone()).unwrap().as_str(), &string);
    }

    #[test]
    fn digest_roundtrip() {
        let mut original_buffer = [0u8; DIGEST_HEX_DIGITS / 2];
        let mut i = 1u64;

        for _ in 0..1000 {
            for b in original_buffer.iter_mut() {
                *b = i as u8;
                i += (3 * i + 27) & 0xFF;
            }

            let sd = DigestString::new_from_binary(&original_buffer).unwrap();

            {
                let reconstructed_buffer = sd.to_bin();
                assert_eq!(original_buffer, reconstructed_buffer);
            }

            {
                let reconstructed_buffer = sd.to_boxed_bin();
                assert_eq!(original_buffer.as_slice(), &reconstructed_buffer[..]);
            }
        }
    }

    #[test]
    fn digest_from_binary_wrong_length() {
        let buffer = [0u8; DIGEST_HEX_DIGITS / 2 - 1];
        assert!(DigestString::new_from_binary(&buffer).is_none());

        let buffer = [0u8; DIGEST_HEX_DIGITS / 2 + 1];
        assert!(DigestString::new_from_binary(&buffer).is_none());
    }
}
