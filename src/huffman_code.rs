// Huffman encoding and decoding.
// https://en.wikipedia.org/wiki/Huffman_coding
// Inspired by https://github.com/pcein/rust-for-fun/blob/master/huffman-coding/tree.rs

use ::std::collections::{HashMap, BinaryHeap};
use std::cmp::{max, min, Ordering};

type Link = Option<Box<Node>>;

#[derive(Eq)]
struct Node {
    freq: i32,
    char: Option<char>,
    right: Link,
    left: Link,
}

// Implement traits to use BinaryHeap as min heap.
impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool { self.freq == other.freq }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering { other.freq.cmp(&self.freq) }
}

pub struct HuffTree {
    root: Link,
    padding: usize,
}

impl HuffTree {
    pub fn new() -> HuffTree {
        HuffTree {
            root: None,
            padding: 0, // Encoded message must satisfy; message.len() % 8 == 0.
        }
    }

    /// Map characters in message to their frequency in message.
    fn create_frequency_map(&mut self, message: &String) -> HashMap<char, i32> {
        let mut frequency_map = HashMap::new();
        for char in message.chars() {
            let count = frequency_map.entry(char).or_insert(0);
            *count += 1;
        }
        frequency_map
    }

    /// Create HuffmanTree to code characters with greater frequency with a short codes and
    /// infrequent characters with long codes.
    ///
    /// Return root of HuffTree.
    fn build_tree(&mut self, frequency_map: HashMap<char, i32>) -> Link {
        // Populate a min heap with single node HuffTrees from the frequency map.
        let mut node_heap: BinaryHeap<Box<Node>> = {
            frequency_map
                .iter()
                .map(|(char, freq)| Box::new(Node {freq: *freq, char: Some(*char), left: None, right: None}))
                .collect()
        };
        // Pop the top two nodes, combine their frequencies to create a new Node with char = None.
        // Assign the larger popped node as the new node's right, the smaller as left and push on the heap.
        // Keep doing this until len is 1. This is the root of the sorted HuffTree.
        while node_heap.len() > 1 {
            let node1 = node_heap.pop().expect("Heap has elements.");
            let node2 = node_heap.pop().expect("Heap has elements.");
            let new_node = Box::new(Node{freq: node1.freq + node2.freq, char:None, left: Some(node1), right: Some(node2)});
            node_heap.push(new_node);
        }
        let root = Some(node_heap.pop().expect("Tree has root."));
        assert!(node_heap.is_empty());
        root
    }

    /// Map characters to their codes by traversing HuffTree.
    ///
    /// Recurse to leaf nodes where characters reside.
    ///
    /// Append to string each step down the path to the char.
    ///
    /// A move to the left appends a '0', to the right a '1'.
    fn assign_codes(&self, tree: &Box<Node>, code_map: &mut HashMap<char, String>, string: String) {
        if let Some(char) = &tree.char {
            code_map.insert(*char, string);
        } else {
            if let Some(left) = &tree.left {
                self.assign_codes(left, code_map, string.clone() + "0");
            }
            if let Some(right) = &tree.right {
                self.assign_codes(right, code_map, string + "1");
            }
        }
    }

    /// Use char_code_map to map characters to their codes.
    ///
    /// Calculate checksum as vec is built. Append u32 checksum to encoded message vec.
    fn encode_string(&mut self, message: &String, char_code_map: HashMap<char, String>) -> Vec<u32> {
        let mut encoded_message: Vec<u32> = Vec::new();
        let mut checksum = 0_u32;
        let mut byte_index = 0_u8;
        for char in message.chars() {
            let code = char_code_map.get(&char).expect("All message chars in map.");
            for bit in code.chars() {
                let bit = bit.to_digit(10).expect("Bits are digits");
                encoded_message.push(bit);
                checksum += bit << byte_index;
                match byte_index {
                    7 => byte_index = 0,
                    _ => byte_index += 1,
                }
            }
        }
        // Pad encoded_message so that encoded_message.len() % 8 == 0.
        self.padding = 8 - (encoded_message.len() % 8);
        for _ in 0..self.padding {
            encoded_message.push(0)
        }
        // Concat with bits from checksum
        let check_vec = (0..32).map(|n| (checksum >> n) & 1).collect();
        Vec::from([encoded_message, check_vec].concat())
    }

    /// Build the tree and encode the message.
    pub fn encode(&mut self, message: String) -> Vec<u32> {
        let frequency_map = self.create_frequency_map(&message);
        self.root = self.build_tree(frequency_map);
        let mut char_code_map = HashMap::new();
        self.assign_codes(
            &self.root.as_ref().expect("Tree exists"),
            &mut char_code_map,
            "".to_string(),
        );
        self.encode_string(&message, char_code_map)
    }

    /// Last 32 bits contain checksum.
    ///
    /// Sum each 8 bit word in message and compare to checksum.
    ///
    /// Return comparison and error.
    fn validate(&self, data: &[u32]) -> (bool, f32) {
        let data_len = data.len();
        // Min one byte message plus checksum.
        if data_len < 40 {
            return (false, 0.0);
        }
        // Sum each u32 byte of data.
        let sum = (0..data_len - 32)
            .step_by(8)
            .fold(0, |byte, i| {
                byte + (0..8)
                .fold(0, |bit, j|
                    bit + ( data[i + j] << j )
                )
            });
        // Get checksum.
        let check = data[data_len - 32..]
            .iter()
            .enumerate()
            .fold(0, |acc, (i, bit)| acc + (*bit << i));
        // VERY roughly estimate data fidelity.
        let min = min(sum, check) as f32;
        let max = max(sum, check) as f32;
        let error = 1.0 - (min / max);
        (error < 0.005, error)
    }

    /// Use encoded message to traverse tree and find characters.
    ///
    /// A '0' moves down the tree to the left, '1' to the right.
    ///
    /// Only leaf nodes have characters so if we found one that's it.
    fn decode_string(&self, encoded_message: &[u32]) -> String {
        let mut decoded_chars: Vec<char> = Vec::new();
        let mut node = self.root.as_ref().expect("Tree has root.");
        for bit in encoded_message {
            if *bit == 0 {
                if let Some(ref left) = &node.left {
                    node = left;
                }
            } else {
                if let Some(ref right) = &node.right {
                    node = right;
                }
            }
            if let Some(char) = node.char {
                decoded_chars.push(char);
                node = self.root.as_ref().expect("Tree has root.");
            }
        }
        decoded_chars.iter().collect()
    }

    /// Decode the message.
    pub fn decode(&self, encoded_message: &[u32]) -> String {
        let (valid, error) = self.validate(&encoded_message);
        if !valid {
            return format!("Error: Invalid data detected. Data Loss: {:.4}%\n", error * 100.0);
        }
        let sans_checksum_padding = &encoded_message[0..(encoded_message.len() - (32 + self.padding))];
        let decoded_message = self.decode_string(sans_checksum_padding);
        format!("Validated message:\n\n{decoded_message}\nData Loss: {:.4}%\n", error * 100.0)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::read_to_string;

    #[test]
    /// Test that the whole deal works.
    fn test_encode_decode() {
        let message = read_to_string("src/test.txt").expect("file exists");
        let mut huff_tree = HuffTree::new();
        let encoded_message = huff_tree.encode(message.clone());
        assert!(message.len() * 8 > encoded_message.len());
        let (valid, error) = huff_tree.validate(&encoded_message);
        let decoded_message = huff_tree.decode(&encoded_message);
        assert_eq!(valid, true);
        assert_eq!(error, 0.0);
        assert_eq!(decoded_message, format!("Validated message:\n\n{message}\nData Loss: {:.4}%\n", error * 100.0))
    }

    #[test]
    fn test_create_frequency_map() {
        let message = "abbccc".to_string();
        let mut huff_tree = HuffTree::new();
        huff_tree.encode(message.clone());
        let expected = HashMap::from([('a', 1), ('b', 2), ('c', 3)]);
        assert_eq!(huff_tree.create_frequency_map(&message), expected)
    }
}
