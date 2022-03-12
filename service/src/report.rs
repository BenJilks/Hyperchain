use libhyperchain::hash::Hash;
use serde::{Serialize, Deserialize};
use std::collections::{HashSet, HashMap};
use std::path::PathBuf;
use std::fs::File;
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use std::error::Error;

const EXPIRE_TIME: Duration = Duration::from_secs(60 * 60 * 24); // 24 hours

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct NodeReport
{
    pub expires: u128,
    pub chunks_stored: HashSet<Hash>,
}

impl NodeReport
{

    pub fn new(chunks_stored: HashSet<Hash>) -> Self
    {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_nanos();

        Self
        {
            expires: now + EXPIRE_TIME.as_nanos(),
            chunks_stored,
        }
    }

}

pub struct Report
{
    path: PathBuf,
    nodes: HashMap<String, NodeReport>,
}

impl Report
{

    fn load_existing_report(path: &PathBuf) 
        -> Result<HashMap<String, NodeReport>, Box<dyn Error>>
    {
        let file = File::open(path)?;
        Ok(serde_json::from_reader(file)?)
    }

    pub fn open(path: &PathBuf) -> Self
    {
        let nodes = Self::load_existing_report(path)
            .unwrap_or(Default::default());

        Self
        {
            path: path.to_owned(),
            nodes,
        }
    }

    fn flush(&self) -> Result<(), Box<dyn Error>>
    {
        let file = File::create(&self.path)?;
        serde_json::to_writer_pretty(file, &self.nodes)?;
        Ok(())
    }

    fn flush_handle_errors(&self)
    {
        match self.flush()
        {
            Ok(_) => {},
            Err(err) => warn!("Could not flush report: {}", err),
        }
    }

    pub fn add(&mut self, address: &str, node_report: NodeReport) -> bool
    {
        let existing_or_none = self.nodes.get(address);
        if existing_or_none.is_some()
        {
            let existing = existing_or_none.unwrap();
            if node_report.expires <= existing.expires {
                return false;
            }
        }

        self.nodes.insert(address.to_owned(), node_report);
        self.flush_handle_errors();
        true
    }

    pub fn update(&mut self) -> impl Iterator<Item = String>
    {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_nanos();

        let expired = self.nodes
            .iter()
            .filter(|(_, report)| now > report.expires)
            .map(|(address, _)| address.to_owned())
            .collect::<Vec<_>>();

        for address in &expired {
            self.nodes.remove(address);
        }

        self.flush_handle_errors();
        expired.into_iter()
    }

    pub fn storage_usage(&self) -> HashMap<Hash, usize>
    {
        let mut usage = HashMap::new();
        for (_, report) in &self.nodes
        {
            for chunk in &report.chunks_stored 
            {
                if !usage.contains_key(chunk) {
                    usage.insert(chunk.clone(), 0);
                }
                *usage.get_mut(chunk).unwrap() += 1;
            }
        }

        usage
    }

}

#[cfg(test)]
mod test
{

    use super::*;
    use libhyperchain::config::HASH_LEN;
    use libhyperchain::wallet::private_wallet::PrivateWallet;
    use libhyperchain::transaction::Transaction;
    use libhyperchain::transaction::page::Page;
    use libhyperchain::data_store::data_unit::DataUnit;
    use libhyperchain::data_store::page::CreatePageData;
    use crate::node::packet_handler::NodePacketHandler;
    use crate::node::tests::{create_node, mine_block, wait_for_block};
    use crate::network::NetworkConnection;
    use crate::network::packet::Packet;
    use std::iter::FromIterator;

    #[test]
    fn test_report()
    {
        let _ = pretty_env_logger::try_init();

        let chunk_a = Hash::from(&[0u8; HASH_LEN]);
        let chunk_b = Hash::from(&[1u8; HASH_LEN]);
        let chunk_c = Hash::from(&[2u8; HASH_LEN]);

        let mut report = Report::open(&std::env::temp_dir().join("report.json"));
        report.add("a", NodeReport::new(HashSet::from_iter([chunk_a, chunk_b, chunk_c])));
        report.add("b", NodeReport::new(HashSet::from_iter([chunk_a, chunk_c])));

        let mut node_c_report = NodeReport::new(HashSet::from_iter([chunk_b]));
        node_c_report.expires -= EXPIRE_TIME.as_nanos() + 1;
        report.add("c", node_c_report);

        assert_eq!(report.storage_usage(), 
                   HashMap::from_iter([(chunk_a, 2), (chunk_b, 2), (chunk_c, 2)]));

        assert_eq!(report.update().collect::<Vec<_>>(), vec!["c"]);
        assert_eq!(report.storage_usage(), 
                   HashMap::from_iter([(chunk_a, 2), (chunk_b, 1), (chunk_c, 2)]));
    }

    fn create_page(connection: &mut NetworkConnection<NodePacketHandler>,
                   wallet: &PrivateWallet) -> (Transaction<Page>, DataUnit)
    {
        // TODO: Not allow pages with no name.
        let data_unit = DataUnit::CreatePage(
            CreatePageData::new("test".to_owned(), vec![0, 1, 2]));

        let (page, new_report) =
        {
            let mut node = connection.handler().node();
            let chain = &mut node.chain();
            let page = chain.new_page(wallet, &data_unit, 1.0)
                .expect("Error creating page");

            node.data_store().store_data_unit(&data_unit)
                .expect("Error storing page data");

            let new_report = node.our_report()
                .expect("Unable to create report");

            (page, new_report)
        };

        connection.manager().send(Packet::Report(None, new_report))
            .expect("Unable to broadcast new report");

        (page, data_unit)
    }

    fn get_storage_usage(connection: &NetworkConnection<NodePacketHandler>
                         ) -> HashMap<Hash, usize>
    {
        let node = connection.handler().node();
        node.storage_usage()
            .expect("Couldn't get storage usage")
    }

    fn wait_for(condition: impl Fn() -> bool, timeout: u32) -> Result<(), ()>
    {
        let mut timer = 0;
        while !condition()
        {
            std::thread::sleep(Duration::from_millis(100));

            timer += 100;
            if timer >= timeout {
                return Err(());
            }
        }

        Ok(())
    }

    #[test]
    fn test_report_sync()
    {
        let _ = pretty_env_logger::try_init();

        let wallet = PrivateWallet::open_temp(0).unwrap();
        let mut connection_a = create_node(8040);
        let mut connection_b = create_node(8041);
        connection_b.manager().register_node("127.0.0.1:8040");

        // Ensure nodes are connected.
        mine_block(&mut connection_a, &wallet);
        wait_for_block(&connection_b, 0);

        let (test_page, test_data) = create_page(&mut connection_a, &wallet);
        connection_a.manager().send(Packet::Page(test_page, test_data))
            .expect("Failed to send page request");

        let storage_usage_a = get_storage_usage(&connection_a);
        assert_eq!(storage_usage_a.len(), 1);

        wait_for(|| get_storage_usage(&connection_b).len() == 1, 1000)
            .expect("Node B did not receive page");
        wait_for(|| get_storage_usage(&connection_b).values().next() == Some(&2), 1000)
            .expect("Node B did not receive report");

        let mut connection_c = create_node(8042);
        connection_c.manager().register_node("127.0.0.1:8041");

        // Ensure C is connected to B
        mine_block(&mut connection_b, &wallet);
        wait_for_block(&connection_c, 1);

        wait_for(|| get_storage_usage(&connection_c).len() == 1, 1000)
            .expect("Node C did not receive reports");

        let storage_usage_c = get_storage_usage(&connection_c);
        assert_eq!(storage_usage_c.values().next(), Some(&2));
    }

}

