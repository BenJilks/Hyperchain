use crate::config::{HASH_LEN, PUB_KEY_LEN};
use serde::{Serialize, Deserialize};
use serde::{Serializer, Deserializer};
use std::fmt;

pub type Signature = HashData<PUB_KEY_LEN>;
pub type Hash = HashData<HASH_LEN>;

#[derive(Hash, PartialEq, Eq, Clone, Copy)]
pub struct HashData<const N: usize>
{
    data: [u8; N]
}

impl<const N: usize> HashData<N>
{

    pub fn empty() -> Self
    {
        Self {
            data: [0u8; N],
        }
    }

    pub fn data(&self) -> &[u8]
    {
        self.data.as_ref()
    }

}

impl<'a, T, const N: usize> From<T> for HashData<N>
    where T: IntoIterator<Item = &'a u8>
{
    fn from(value: T) -> Self
    {
        let mut data = [0u8; N];
        for (i, byte) in value.into_iter().enumerate() {
            data[i] = *byte;
        }

        Self {
            data,
        }
    }
}

impl<const N: usize> Serialize for HashData<N>
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        self.data().to_vec().serialize(serializer)
    }
}

impl<'de, const N: usize> Deserialize<'de> for HashData<N>
{
    fn deserialize<De>(deserializer: De) -> Result<Self, De::Error>
        where De: Deserializer<'de>
    {
        let vec = Vec::<u8>::deserialize(deserializer)?;
        Ok(Self::from(&vec))
    }
}

impl<const N: usize> AsRef<[u8]> for HashData<N>
{
    fn as_ref(&self) -> &[u8]
    {
        self.data.as_ref()
    }
}

impl<const N: usize> fmt::Display for HashData<N>
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        write!(f, "{}", base_62::encode(self.data.as_ref()))
    }
}

impl<const N: usize> fmt::Debug for HashData<N>
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        let hash_str = format!("{}", base_62::encode(self.data.as_ref()));
        if hash_str.len() <= 8 {
            write!(f, "{}", hash_str)
        } else {
            write!(f, "{}…{}", &hash_str[0..4], &hash_str[hash_str.len() - 4..])
        }
    }
}

