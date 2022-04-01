/*
 * Copyright (c) 2022, Ben Jilks <benjyjilks@gmail.com>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

package blockchain

import (
	"math"
)

const BlockTime = 10 * 1000 // 10 second blocks
const BlockSampleSize = 100

const TargetLen = 4
type Target [TargetLen]byte

func MinTarget() Target {
    return Target { 0x00, 0xFF, 0xFF, 0x20 }
}

func index(target Target) uint32 {
    // NOTE: Not sure if an index of > 0x20 should be an error.
    index := target[3]
    if index > 0x20 {
        return 0x20
    } else {
        return uint32(index)
    }
}

func coefficent(target Target) uint32 {
    result := uint32(0)
    coefficent_len := TargetLen - 1
    for i := 0; i < coefficent_len; i++ {
        result |= uint32(target[i]) << (coefficent_len - i - 1)*8
    }

    return result
}

func difficulty(target Target) float64 {
    exponent_diff := float64(8 * (index(MinTarget()) - index(target)))
    coefficent_diff := float64(coefficent(MinTarget())) / float64(coefficent(target))
    return coefficent_diff * math.Exp2(exponent_diff)
}

func hashRate(diff float64, time uint64) float64 {
    return float64(diff * 256.0 * BlockSampleSize) / float64(time)
}

func diffForHashRate(hashRate float64) float64 {
    return float64(hashRate * BlockTime) / 256.0
}

func compactFromDifficulty(diff float64) Target {
    exponent := math.Round(math.Log2(diff))
    offset_diff := diff / math.Exp2(exponent)

    id := byte((256.0 - exponent) / 8.0)
    co := uint32(float64(coefficent(MinTarget())) / offset_diff)

    return Target { byte(co >> 16), byte(co >> 8), byte(co), id }
}

func hashFromTarget(compact Target) [32]byte {
    target := [32]byte{}
    start := 32 - index(compact)

    if start > 32 - 3 {
        return target
    }

    for i := uint32(0); i < 3; i++ {
        target[start + i] = compact[i];
    }

    return target
}

func CalculateTarget(sample_start_or_none *Block,
                     sample_end_or_none *Block) Target {
    // If we do not have enough data for a sample, use the min target
    if sample_start_or_none == nil || sample_end_or_none == nil {
        return MinTarget()
    }

    // We're within the sample range, so keep the last target
    sample_end := *sample_end_or_none
    if sample_end.Id % BlockSampleSize != 0 {
        return sample_end.Target
    }

    // Calculate new target with sample
    sample_start := *sample_start_or_none
    sample_time := sample_end.Timestamp - sample_start.Timestamp
    curr_diff := difficulty(sample_end.Target)
    curr_hash_rate := hashRate(curr_diff, sample_time)

    new_diff := diffForHashRate(curr_hash_rate)
    return compactFromDifficulty(new_diff)
}

func IsValidHashForTarget(hash [32]byte, target Target) bool {
    fullTarget := hashFromTarget(target)
    for i := 0; i < 32; i++ {
        if hash[i] < fullTarget[i] {
            return true
        }
        if hash[i] > fullTarget[i] {
            return false
        }
    }

    return true
}

