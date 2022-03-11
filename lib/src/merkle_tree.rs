use crate::hash::Hash;
use sha2::{Sha256, Digest};

type Node = Vec<u8>;

fn reduce_nodes(nodes: Vec<Node>) -> Vec<Node>
{
    nodes
        .chunks(2)
        .map(|pair|
        {
            if pair.len() == 1 {
                return pair[0].clone();
            }

            let mut hasher = Sha256::default();
            hasher.update(&pair[0]);
            hasher.update(&pair[1]);
            hasher.finalize().to_vec()
        })
        .collect::<Vec<_>>()
}

pub fn calculate_merkle_root<H>(data: &[H]) -> Hash
    where H: AsRef<[u8]>
{
    if data.len() == 0 {
        return Hash::empty();
    }

    let mut nodes = data
        .iter()
        .map(|x| 
        {
            let mut hasher = Sha256::default();
            hasher.update(&x);
            hasher.finalize().to_vec()
        })
        .collect::<Vec<_>>();

    while nodes.len() != 1 {
        nodes = reduce_nodes(nodes);
    }

    let root = &nodes[0];
    Hash::from(root)
}

#[cfg(test)]
mod tests
{

    use super::*;

    #[test]
    fn test_merkle_tree()
    {
        let a = vec![1, 2];
        let b = vec![6, 4, 7];
        let c = vec![0];

        let expected_hash = Hash::from(
            &[55, 96, 134, 211, 95, 199, 179, 17, 11, 240, 249, 
            17, 155, 238, 27, 49, 204, 189, 47, 127, 98, 20, 
            247, 132, 161, 187, 217, 199, 158, 253, 81, 231]);

        {
            let merkle_root = calculate_merkle_root(&[&a, &b, &c]);
            assert_eq!(merkle_root, expected_hash);
        }

        {
            let merkle_root = calculate_merkle_root(&[&a, &c, &b]);
            assert_ne!(merkle_root, expected_hash);
        }

        assert_eq!(calculate_merkle_root::<Vec<u8>>(&[]), Hash::empty());
    }

}
