use sha2::{Digest, Sha256};

use crate::{Error, Result};

pub(crate) struct MerkleTree {
    root: MerkleNode,
}

impl MerkleTree {
    pub(crate) fn new(mut datas: Vec<Vec<u8>>) -> Result<MerkleTree> {
        if datas.is_empty() {
            return Err(Error::CustomError("Datas is empty!".to_owned()));
        }

        if datas.len() % 2 != 0 {
            datas.push(datas[0].clone());
        }

        let mut nodes: Vec<MerkleNode> = datas
            .into_iter()
            .map(|data| MerkleNode::new(None, None, Some(data)))
            .collect();

        let mut level = nodes.clone();
        while level.len() > 1 {
            nodes = Vec::new();
            for chunk in level.chunks(2) {
                let left = if !chunk.is_empty() {
                    Some(chunk[0].clone())
                } else {
                    None
                };
                let right = if chunk.len() > 1 {
                    Some(chunk[1].clone())
                } else {
                    None
                };
                nodes.push(MerkleNode::new(left, right, None));
            }
            level = nodes.clone();
        }

        Ok(MerkleTree {
            root: level[0].clone(),
        })
    }

    pub(crate) fn root_hash(&self) -> Vec<u8> {
        self.root.hash.clone()
    }
}

#[derive(Clone)]
struct MerkleNode {
    left: Option<Box<MerkleNode>>,
    right: Option<Box<MerkleNode>>,
    hash: Vec<u8>,
}
impl MerkleNode {
    fn new(
        left: Option<MerkleNode>,
        right: Option<MerkleNode>,
        data: Option<Vec<u8>>,
    ) -> MerkleNode {
        let mut node = MerkleNode {
            left: None,
            right: None,
            hash: vec![],
        };

        if let Some(data) = data {
            node.hash = Sha256::digest(&data).to_vec();
        } else {
            let mut hasher = Sha256::new();
            if let Some(left_node) = left.clone() {
                hasher.update(&left_node.hash);
            }
            if let Some(right_node) = right.clone() {
                hasher.update(&right_node.hash);
            }
            node.hash = hasher.finalize().to_vec();
            node.left = left.map(Box::new);
            node.right = right.map(Box::new);
        }

        node
    }
}
