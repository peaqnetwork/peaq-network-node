use peaq_dev_runtime;

/// An index to a block.
pub type BlockNumber = peaq_dev_runtime::BlockNumber;

/// Header type.
pub type Header = peaq_dev_runtime::Header;

/// Block type.
pub type Block = peaq_dev_runtime::opaque::Block;

/// A hash of some data used by the chain.
pub type Hash = peaq_dev_runtime::Hash;

/// Balance of an account.
pub type Balance = peaq_dev_runtime::Balance;

/// Index of a transaction in the chain.
pub type Nonce = u32;

/// Some way of identifying an account on the chain.
pub type AccountId = peaq_dev_runtime::AccountId;

pub type Index = peaq_dev_runtime::Index;
