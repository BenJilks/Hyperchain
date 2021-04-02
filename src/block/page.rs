use super::{Signature, BlockChain};

#[derive(Debug, Clone)]
pub struct Page
{
    pub page_data: Vec<u8>,
    pub site_id: Signature,
    pub page_name: [u8; 64],
    pub page_fee: u32,
    pub signature: Signature,
}

impl Page
{

    pub fn new(diff: Vec<u8>, site_id: Signature, page_name: [u8; 64], fee: u32, signature: Signature) -> Self
    {
        Self
        {
            page_data: diff,
            site_id: site_id,
            page_name: page_name,
            page_fee: fee,
            signature: signature,
        }
    }

    pub fn from_page(chain: BlockChain, new_page: Vec<u8>, site_id: Signature, page_name: [u8; 64], fee: u32, signature: Signature) -> Self
    {
        let diff = Vec::<u8>::new(); // TODO: Calculate diff
        Self::new(diff, site_id, page_name, fee, signature)
    }

}
