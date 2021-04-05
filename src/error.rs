use std::string::ToString;

#[derive(Debug)]
pub enum Error
{
    NotNextBlock,
    PrevNone,
    PrevInvalidHash,
    BlockTooLarge,
    NoValidBranches,
    DuplicateBlock,

    InvalidPOW,
    InvalidTimestamp,
    InvalidTarget,
    InvalidTransactionSignature,
    InvalidPageSignature,
    InvalidBalance,

    Other(String),
}

impl ToString for Error
{

    fn to_string(&self) -> String
    {
        match self
        {
            Self::NotNextBlock => "Not next block",
            Self::PrevNone => "Could not find prev block",
            Self::PrevInvalidHash => "Prev hash does not match hash in block",
            Self::BlockTooLarge => "Block too large",
            Self::NoValidBranches => "No valid branches",
            Self::DuplicateBlock => "Block already in chain",

            Self::InvalidPOW => "Invalid block (POW)",
            Self::InvalidTimestamp => "Invalid timestamp",
            Self::InvalidTarget => "Invalid target",
            Self::InvalidTransactionSignature => "Invalid transaction signature",
            Self::InvalidPageSignature => "Invalid page signature",
            Self::InvalidBalance => "Invalid balance",
            
            Self::Other(msg) => msg,
        }.to_owned()
    }

}
