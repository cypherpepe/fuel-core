//! The module for all possible coins.

use crate::{
    fuel_asm::Word,
    fuel_tx::{
        Address,
        MessageId,
    },
    fuel_types::AssetId,
};
use coin::Coin;
use deposit_coin::DepositCoin;
use fuel_vm_private::prelude::UtxoId;

pub mod coin;
pub mod deposit_coin;

/// Whether a coin has been spent or not
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Default, Debug, Copy, Clone, Eq, PartialOrd, PartialEq)]
#[repr(u8)]
pub enum CoinStatus {
    /// Coin has not been spent
    Unspent,
    #[default]
    /// Coin has been spent
    Spent,
}

/// The enum of all kind of coins.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum Coins {
    /// The regular coins generated by the transaction output.
    Coin(Coin),
    /// The bridged coin from the DA layer.
    DepositCoin(DepositCoin),
}

/// The unique identifier of the coin.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Copy, Clone, Eq, PartialOrd, PartialEq)]
enum CoinId {
    /// The UTXO id of the regular coin.
    UtxoId(UtxoId),
    /// The unique `nonce` of the `DepositCoin`.
    MessageId(Word),
}

impl Coins {
    /// Returns the coin unique identifier.
    pub fn coin_id(&self) -> CoinId {
        match self {
            Coins::Coin(coin) => CoinId::UtxoId(coin.utxo_id),
            Coins::DepositCoin(coin) => CoinId::MessageId(coin.nonce),
        }
    }

    /// Returns the owner of the coin.
    pub fn owner(&self) -> &Address {
        match self {
            Coins::Coin(coin) => &coin.owner,
            Coins::DepositCoin(coin) => &coin.recipient,
        }
    }

    /// Returns the amount of the asset held by the coin.
    pub fn amount(&self) -> Word {
        match self {
            Coins::Coin(coin) => coin.amount,
            Coins::DepositCoin(coin) => coin.amount,
        }
    }

    /// Returns the asset held by the coin.
    pub fn asset_id(&self) -> &AssetId {
        match self {
            Coins::Coin(coin) => &coin.asset_id,
            Coins::DepositCoin(_) => &AssetId::BASE,
        }
    }

    /// Returns the status of the coin.
    pub fn status(&self) -> CoinStatus {
        match self {
            Coins::Coin(coin) => coin.status,
            Coins::DepositCoin(coin) => coin.status,
        }
    }
}

impl From<Coin> for Coins {
    fn from(coin: Coin) -> Self {
        Coins::Coin(coin)
    }
}

impl From<DepositCoin> for Coins {
    fn from(coin: DepositCoin) -> Self {
        Coins::DepositCoin(coin)
    }
}
