mod private_wallet;
mod public_wallet;
pub use private_wallet::PrivateWallet;
pub use public_wallet::PublicWallet;
use crate::block::{PUB_KEY_LEN, HASH_LEN};

use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WalletStatus
{
    pub balance: f64,
    pub max_id: u32,
}

pub trait Wallet
{

    fn get_public_key(&self) -> [u8; PUB_KEY_LEN];

    fn get_address(&self) -> [u8; HASH_LEN]
    {
        let mut hasher = Sha256::default();
        hasher.update(&self.get_public_key());

        let hash = hasher.finalize();
        *slice_as_array!(&hash, [u8; HASH_LEN]).unwrap()
    }

}
