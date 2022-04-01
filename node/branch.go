/*
 * Copyright (c) 2022, Ben Jilks <benjyjilks@gmail.com>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

package node

import . "hyperchain/blockchain"

type Branch struct {
    blocks map[uint64]Block
    top uint64
    bottom uint64
}

func NewBranch(block Block) Branch {
    branch := Branch {
        blocks: make(map[uint64]Block),
        top: block.Id,
        bottom: block.Id,
    }

    branch.blocks[block.Id] = block
    return branch
}

func (branch *Branch) Add(block Block) bool {
    top := branch.blocks[branch.top]
    bottom := branch.blocks[branch.bottom]

    if block.IsNextTo(top) {
        branch.top += 1
        branch.blocks[block.Id] = block
        return true
    }

    if bottom.IsNextTo(block) {
        branch.bottom -= 1
        branch.blocks[block.Id] = block
        return true
    }

    return false
}

type BlockTree struct {
    branches []Branch
}

func NewBlockTree() BlockTree {
    return BlockTree {
        branches: make([]Branch, 0),
    }
}

func (tree *BlockTree) branchesToMerge() (int, int) {
    for i := range tree.branches {
        branch := &tree.branches[i]

        for j := range tree.branches {
            if i == j {
                continue
            }

            // TODO: Check with `IsNextTo`.

            other := &tree.branches[j]
            if branch.top + 1 == other.bottom {
                return i, j
            }
            if branch.bottom - 1 == other.top {
                return i, j
            }
        }
    }

    return -1, -1
}

func (tree *BlockTree) merge() {
    for i, j := tree.branchesToMerge(); i != -1 && j != -1; {
        branch := &tree.branches[i]
        other := tree.branches[j]
        tree.branches = append(tree.branches[:j], tree.branches[j+1:]...)

        if other.top > branch.top {
            branch.top = other.top
        }
        if other.bottom < branch.bottom {
            branch.bottom = other.bottom
        }
        for id, block := range other.blocks {
            branch.blocks[id] = block
        }
    }
}

func (tree *BlockTree) Add(block Block) {
    for i := range tree.branches {
        branch := &tree.branches[i]
        if branch.Add(block) {
            tree.merge()
            return
        }
    }

    newBranch := NewBranch(block)
    tree.branches = append(tree.branches, newBranch)
}

func (tree *BlockTree) branchToMergeWithChain(chain *BlockChain) int {
    topId := uint64(0)
    if top := chain.Top(); top != nil {
        topId = top.Id
    }

    for i, branch := range tree.branches {
        if branch.bottom > topId {
            continue
        }
        if topId == 0 && branch.bottom == 0 {
            return i
        }

        bottom := branch.blocks[branch.bottom]
        if err := chain.ValidateBlock(bottom); err == nil {
            return i
        }
    }

    return -1
}

func (tree *BlockTree) CanMergeWithChain(chain *BlockChain) bool {
    return tree.branchToMergeWithChain(chain) != -1
}

