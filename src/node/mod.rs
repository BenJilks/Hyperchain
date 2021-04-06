mod network;
mod broadcast;
use crate::miner;
use crate::block::{Block, Transaction, BlockChain};
use crate::wallet::{PrivateWallet, PublicWallet, Wallet};
use crate::error::Error;
use network::{NetworkConnection, Packet};

use std::sync::mpsc::{channel, Sender, Receiver};
use std::sync::{Mutex, Arc};
use std::path::PathBuf;
use std::time::{Instant, Duration};

pub struct Node
{
    connection: Arc<Mutex<NetworkConnection>>,
    top: u64,
    blocks_being_mined: i32,
    at_top: bool,
    start_prune_timer: Instant,

    transaction_queue: Vec<Transaction>,
}

impl Node
{

    pub fn new(port: i32, known_nodes: PathBuf) -> Self
    {
        let connection = NetworkConnection::new(port, known_nodes).unwrap();
        Self
        {
            connection: Arc::from(Mutex::from(connection)),
            top: 0,
            blocks_being_mined: 0,
            at_top: false,
            start_prune_timer: Instant::now(),

            transaction_queue: Vec::new(),
        }
    }

    pub fn add_known_node(&mut self, address: &str)
    {
        self.connection.lock().unwrap().update_known_nodes(address);
    }

    fn miner_thread(to_mine: Receiver<Block>, done: Sender<Block>)
    {
        for block in to_mine {
            done.send(miner::mine_block(block)).expect("worked");
        }
    }

    fn process_received_block(&mut self, chain: &mut BlockChain, block: Block)
    {
        println!("Got block {}", block.block_id);
        match chain.add(&block)
        {
            Ok(_) => self.top = chain.top_id(),
            Err(Error::NoValidBranches) => self.top -= 1,
            Err(Error::DuplicateBlock) => {},
            Err(_) => panic!(),
        }
        NetworkConnection::set_top(&mut self.connection, self.top);
    }

    fn process_received_transaction(&mut self, chain: &mut BlockChain, transaction: Transaction)
    {
        println!("Got tranaction {}", transaction.to_string());
        let wallet = PublicWallet::from_public_key_e(transaction.header.from, transaction.e);
        let balance = wallet.calculate_balance(chain.longest_branch());

        if balance < transaction.header.amount + transaction.header.transaction_fee 
            || !wallet.varify(&transaction.header.hash().unwrap(), &transaction.signature)
        {
            NetworkConnection::broadcast(&mut self.connection, None, Packet::TransactionRequestRejected(transaction));
            return;
        }

        self.transaction_queue.push(transaction.clone());
        NetworkConnection::broadcast(&mut self.connection, None, Packet::TransactionRequestAccepted(transaction));
    }

    fn process_packets(&mut self, chain: &mut BlockChain)
    {
        for packet in NetworkConnection::process_packets(&mut self.connection)
        {
            match packet
            {
                Packet::NewBlock(block) => self.process_received_block(chain, block),
                Packet::TransactionRequest(transaction) => self.process_received_transaction(chain, transaction),
                Packet::TransactionRequestAccepted(transaction) => println!("Accepted {}", transaction.to_string()),
                Packet::TransactionRequestRejected(transaction) => println!("Rejected {}", transaction.to_string()),
                _ => panic!(),
            }
        }
    }

    fn process_mined_blocks(&mut self, chain: &mut BlockChain, blocks_done_recv: &Receiver<Block>)
    {
        for block in blocks_done_recv.try_iter() 
        {
            self.blocks_being_mined -= 1;
            if block.block_id != chain.top_id() + 1 
            {
                println!("Mined block {} not at top {}", block.block_id, chain.top_id());
                continue;
            }
            if block.validate(chain.longest_branch()).is_err() 
            {
                println!("Mined block not valid on longest branch");
                continue;
            }
            if chain.add(&block).is_err() {
                continue;
            }
            
            println!("Accepted our block {}!!", block.block_id);
            self.transaction_queue.clear();
            self.top = chain.top_id();
            NetworkConnection::set_top(&mut self.connection, self.top);
        }
    }

    fn process_nodes(&mut self, chain: &mut BlockChain)
    {
        self.at_top = true;
        for (address, node) in NetworkConnection::nodes(&mut self.connection) 
        {
            if node.top < self.top 
            {
                let block = chain.longest_branch().block(node.top + 1);
                if block.is_some()
                {
                    NetworkConnection::broadcast(&mut self.connection, 
                        Some( address ), Packet::NewBlock(block.unwrap()));
                }
            }

            if node.top > self.top {
                self.at_top = false;
            }
        }
    }

    fn process_new_blocks_to_mine(&mut self, chain: &mut BlockChain, wallet: &PrivateWallet, blocks_to_mine_send: &Sender<Block>)
    {
        if self.at_top && self.blocks_being_mined == 0
        {
            let mut block = Block::new(chain.longest_branch(), wallet).expect("Can make block");
            for transaction in &self.transaction_queue 
            {
                println!("Adding {} to block {}", transaction.to_string(), block.block_id);
                block.add_transaction(transaction.clone());
            }

            self.blocks_being_mined += 1;
            blocks_to_mine_send.send(block).expect("Worked");
        }
    }

    fn prune_branches(&mut self, chain: &mut BlockChain)
    {
        if self.start_prune_timer.elapsed().as_secs() >= 1 
        {
            chain.prune_branches();
            self.start_prune_timer = Instant::now();
        }
    }

    fn make_transaction(&mut self, transaction: Transaction)
    {
        NetworkConnection::broadcast(&mut self.connection, None, 
            Packet::TransactionRequest(transaction.clone()));
        
        self.transaction_queue.push(transaction);
    }

    pub fn run(&mut self, chain: &mut BlockChain, wallet: &PrivateWallet)
    {
        self.top = chain.top_id();
        NetworkConnection::run(self.connection.clone());
        NetworkConnection::set_top(&mut self.connection, self.top);

        let (blocks_to_mine_send, blocks_to_mine_recv) = channel::<Block>();
        let (blocks_done_send, blocks_done_recv) = channel::<Block>();
        std::thread::spawn(move || {
            Self::miner_thread(blocks_to_mine_recv, blocks_done_send);
        });

        let other = PrivateWallet::read_from_file(&std::path::PathBuf::from("other.wallet")).unwrap();
        self.make_transaction(Transaction::for_block(chain.longest_branch(), wallet, &other, 10.0, 1.0).unwrap());

        loop
        {
            self.process_packets(chain);
            self.process_mined_blocks(chain, &blocks_done_recv);
            self.process_nodes(chain);
            self.process_new_blocks_to_mine(chain, wallet, &blocks_to_mine_send);
            self.prune_branches(chain);

            std::thread::sleep(Duration::from_millis(100));
        }
    }

}
