use crate::block::Block;
use crate::error::Error;

pub const BUFFER_SIZE: usize = 3;

pub struct BlockBuffer
{
    buffer: [Vec<Block>; BUFFER_SIZE],
    base_id: Option<u64>,
}

impl BlockBuffer
{

    pub fn new() -> Self
    {
        Self
        {
            buffer: Default::default(),
            base_id: None,
        }
    }

    pub fn base_id(&self) -> Option<u64>
    {
        self.base_id
    }

    fn follow_branch(&self, block: &Block, depth: usize) -> (usize, Vec<Block>)
    {
        if depth >= BUFFER_SIZE {
            return (depth - 1, vec![block.clone()]);
        }

        let mut max_depth = depth - 1;
        let mut max_branch = Vec::<Block>::new();
        for next in &self.buffer[depth]
        {
            if next.is_next_block(block).is_ok() 
            {
                let branch = self.follow_branch(next, depth + 1);
                max_depth = std::cmp::max(max_depth, branch.0);
                max_branch = branch.1;
            }
        }

        let mut branch = vec![block.clone()];
        branch.append(&mut max_branch);

        (max_depth, branch)
    }

    fn find_max_branch(&self) -> Vec<Block>
    {
        let mut max_depth = 0;
        let mut max_branch = Vec::<Block>::new();
        for block in &self.buffer[0] 
        {
            let branch = self.follow_branch(block, 1);
            if branch.0 >= max_depth
            {
                max_depth = branch.0;
                max_branch = branch.1;
            }
        }

        max_branch
    }

    fn clear_buffer(&mut self)
    {
        self.base_id = None;
        for it in &mut self.buffer {
            it.clear();
        }
    }

    fn shift_buffer(&mut self, block: Block) -> Option<Block>
    {
        let max_branch = self.find_max_branch();
        if max_branch.is_empty()
        {
            self.clear_buffer();
            return None;
        }
        
        *self.base_id.as_mut().unwrap() += 1;
        for i in 1..BUFFER_SIZE {
            self.buffer[i - 1] = self.buffer[i].clone();
        }
        self.buffer[BUFFER_SIZE - 1].push(block);

        Some( max_branch[0].clone() )
    }

    pub fn push(&mut self, block: Block) -> Result<Option<Block>, Error>
    {
        if self.base_id.is_none() 
        {
            self.base_id = Some( block.block_id );
            self.buffer[0].push(block);
            return Ok(None);
        }

        if block.block_id == self.base_id.unwrap() + BUFFER_SIZE as u64 {
            return Ok(self.shift_buffer(block));
        }

        if block.block_id < self.base_id.unwrap() || 
            block.block_id > self.base_id.unwrap() + BUFFER_SIZE as u64 
        {
            return Err(Error::NotNextBlock);
        }

        let buffer_index = (block.block_id - self.base_id.unwrap()) as usize;
        self.buffer[buffer_index].push(block);
        Ok(None)
    }

    pub fn top(&self) -> Option<Block>
    {
        if self.base_id.is_none() {
            return None;
        }

        let max_branch = self.find_max_branch();
        match max_branch.last()
        {
            Some(block) => Some( block.clone() ),
            None => None,
        }
    }

    pub fn block(&self, block_id: u64) -> Option<Block>
    {
        if self.base_id.is_none() {
            return None;
        }

        let base_id = self.base_id.unwrap();
        let max_branch = self.find_max_branch();
        if block_id < base_id || block_id >= base_id + max_branch.len() as u64 {
            return None;
        }

        Some( max_branch[(block_id - base_id) as usize].clone() )
    }

}
