use super::HASH_LEN;
use super::Block;

const TARGET_LEN: usize = 4;
const MIN_TARGET: [u8; TARGET_LEN] = [0x00, 0xFF, 0xFF, 0x20];
pub type Target = [u8; TARGET_LEN];

const BLOCK_TIME: u64 = 1000;
pub const BLOCK_SAMPLE_SIZE: u64 = 10;

fn index(target: &Target) -> u32
{
    // NOTE: Not sure if an index of > 0x20 should be an error.
    std::cmp::min(target[3] as u32, 0x20)
}

fn coefficent(target: &Target) -> u32
{
    let mut result = 0u32;
    let coefficent_len = TARGET_LEN - 1;
    for i in 0..coefficent_len {
        result |= (target[i] as u32) << (coefficent_len - i - 1)*8;
    }

    result
}

pub fn difficulty(target: &Target) -> f64
{
    let exponent_diff = (8 * (index(&MIN_TARGET) - index(target))) as f64;
    let coefficent_diff = coefficent(&MIN_TARGET) as f64 / coefficent(target) as f64;
    coefficent_diff * exponent_diff.exp2()
}

fn hash_rate(diff: f64, time: u64) -> f64
{
    (diff * 256.0 * BLOCK_SAMPLE_SIZE as f64) / time as f64
}

fn diff_for_hash_rate(hash_rate: f64) -> f64
{
    (hash_rate * BLOCK_TIME as f64) / 256.0
}

fn compact_from_difficulty(diff: f64) -> Target
{
    let exponent = diff.log2().round();
    let offset_diff = diff / exponent.exp2();

    let id = ((256.0 - exponent) / 8.0) as u8;
    let co = (coefficent(&MIN_TARGET) as f64 / offset_diff) as u32;

    [
        (co >> 16) as u8,
        (co >> 8) as u8,
        co as u8,
        id,
    ]
}

pub fn hash_from_target(compact: &Target) -> [u8; HASH_LEN]
{
    let mut target = [0u8; HASH_LEN];

    let start = HASH_LEN - index(compact) as usize;
    for i in 0..3 {
        target[start + i] = compact[i];
    }

    target
}

pub fn calculate_target(sample_start_or_none: Option<Block>, 
                        sample_end_or_none: Option<Block>) -> Target
{
    // If we do not have enough data for a sample, use the min target
    if sample_start_or_none.is_none() || sample_end_or_none.is_none() {
        return MIN_TARGET;
    }

    // We're within the sample range, so keep the last target
    let sample_end = sample_end_or_none.unwrap();
    if sample_end.block_id % BLOCK_SAMPLE_SIZE != 0 {
        return sample_end.target;
    }

    // Calculate new target with sample
    let sample_start = sample_start_or_none.unwrap();
    let sample_time = sample_end.timestamp - sample_start.timestamp;
    let curr_diff = difficulty(&sample_end.target);
    let curr_hash_rate = hash_rate(curr_diff, sample_time as u64);

    let new_diff = diff_for_hash_rate(curr_hash_rate);
    compact_from_difficulty(new_diff)
}

#[cfg(test)]
mod tests
{

    use super::*;

    #[test]
    fn test_target_calc()
    {
        assert_eq!(
            hash_from_target(&MIN_TARGET), 
            [
                0x00, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            ]);
        assert_eq!(
            hash_from_target(&[0x12, 0x34, 0x56, 0x14]),
            [
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x12, 0x34, 0x56, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            ]);

        assert_eq!(difficulty(&MIN_TARGET), 1.0);
        assert_eq!(compact_from_difficulty(1.0), MIN_TARGET);

        assert_eq!(difficulty(&[0x00, 0xFF, 0xFF, 0x1F]), 256.0);
        assert_eq!(compact_from_difficulty(256.0), [0x00, 0xFF, 0xFF, 0x1F]);

        // NOTE: Not exact, but hopefully good enough for now
        assert_eq!(difficulty(&[0x00, 0x12, 0x34, 0x1F]), 3600.206008583691);
        assert_eq!(compact_from_difficulty(3600.206008583691), [0x01, 0x23, 0x40, 0x1E]);

        // NOTE: Not exact, but hopefully good enough for now
        assert_eq!(difficulty(&[0x00, 0xEE, 0xEE, 0x10]), 364588250272434060000000000000000000000.0);
        assert_eq!(compact_from_difficulty(364588250272434406000000000000000000000.0), [0x00, 0xEE, 0xED, 0x10]);

        assert_eq!(hash_rate(1.0, BLOCK_SAMPLE_SIZE), 256.0);
        assert_eq!(hash_rate(difficulty(&[0x00, 0xFF, 0xFF, 0x1F]), BLOCK_SAMPLE_SIZE), 65536.0);

        assert_eq!(diff_for_hash_rate(256.0), BLOCK_TIME as f64);
    }

}
