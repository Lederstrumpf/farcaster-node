// LNP Node: node running lightning network protocol and generalized lightning
// channels.
// Written in 2020 by
//     Dr. Maxim Orlovsky <orlovsky@pandoracore.com>
//
// To the extent possible under law, the author(s) have dedicated all
// copyright and related and neighboring rights to this software to
// the public domain worldwide. This software is distributed without
// any warranty.
//
// You should have received a copy of the MIT License
// along with this software.
// If not, see <https://opensource.org/licenses/MIT>.

use amplify::{ToYamlString, Wrapper};
use internet2::addr::InetSocketAddr;
#[cfg(feature = "serde")]
use serde_with::{DisplayFromStr, DurationSeconds, Same};
use std::collections::BTreeMap;
use std::fmt::{self, Debug, Display, Formatter};
use std::iter::FromIterator;
use std::time::Duration;

use bitcoin::{secp256k1, OutPoint};
use internet2::{NodeAddr, RemoteSocketAddr};
use lnp::payment::{self, AssetsBalance, Lifecycle};
use lnp::{
    message, ChannelId as SwapId, Messages, TempChannelId as TempSwapId,
};
use lnpbp::chain::AssetId;
use lnpbp::strict_encoding::{StrictDecode, StrictEncode};
use microservices::rpc::Failure;
use microservices::rpc_connection;
use wallet::PubkeyScript;

use farcaster_core::{
    protocol_message::CommitAliceSessionParams,
    bitcoin::Bitcoin,
    monero::Monero,
};

#[derive(Clone, Debug, StrictDecode, StrictEncode)]
#[strict_encoding_crate(lnpbp::strict_encoding)]
#[non_exhaustive]
pub enum FarMsgs {
    CommitAliceSessionParams(CommitAliceSessionParams<Bitcoin, Monero>)
}

use crate::ServiceId;

#[derive(Clone, Debug, Display, From, LnpApi)]
#[encoding_crate(lnpbp::strict_encoding)]
#[lnp_api(encoding = "strict")]
#[non_exhaustive]
pub enum Request {
    #[lnp_api(type = 0)]
    #[display("hello()")]
    Hello,

    #[lnp_api(type = 1)]
    #[display("update_channel_id({0})")]
    UpdateSwapId(SwapId),

    #[lnp_api(type = 2)]
    #[display("send_message({0})")]
    PeerMessage(Messages),

    #[lnp_api(type = 3)]
    #[display("send_message({0})")]
    FarMsgs(FarMsgs),

    // Can be issued from `cli` to `lnpd`
    #[lnp_api(type = 100)]
    #[display("get_info()")]
    GetInfo,

    // Can be issued from `cli` to `lnpd`
    #[lnp_api(type = 101)]
    #[display("list_peers()")]
    ListPeers,

    // Can be issued from `cli` to `lnpd`
    #[lnp_api(type = 102)]
    #[display("list_channels()")]
    ListSwaps,

    // Can be issued from `cli` to `lnpd`
    #[lnp_api(type = 200)]
    #[display("listen({0})")]
    Listen(RemoteSocketAddr),

    // Can be issued from `cli` to `lnpd`
    #[lnp_api(type = 201)]
    #[display("connect({0})")]
    ConnectPeer(NodeAddr),

    // Can be issued from `cli` to a specific `peerd`
    #[lnp_api(type = 202)]
    #[display("ping_peer()")]
    PingPeer,

    // Can be issued from `cli` to `lnpd`
    #[lnp_api(type = 203)]
    #[display("create_channel_with(...)")]
    OpenSwapWith(CreateSwap),

    #[lnp_api(type = 204)]
    #[display("accept_channel_from(...)")]
    AcceptSwapFrom(CreateSwap),

    #[lnp_api(type = 205)]
    #[display("fund_channel({0})")]
    FundSwap(OutPoint),

    /* TODO: Activate after lightning-invoice library update
    // Can be issued from `cli` to a specific `peerd`
    #[lnp_api(type = 208)]
    #[display("pay_invoice({0})")]
    PayInvoice(Invoice),
     */
    // Responses to CLI
    // ----------------
    #[lnp_api(type = 1002)]
    #[display("progress({0})")]
    Progress(String),

    #[lnp_api(type = 1001)]
    #[display("success({0})")]
    Success(OptionDetails),

    #[lnp_api(type = 1000)]
    #[display("failure({0:#})")]
    #[from]
    Failure(Failure),

    #[lnp_api(type = 1100)]
    #[display("node_info({0})", alt = "{0:#}")]
    #[from]
    NodeInfo(NodeInfo),

    #[lnp_api(type = 1101)]
    #[display("node_info({0})", alt = "{0:#}")]
    #[from]
    PeerInfo(PeerInfo),

    #[lnp_api(type = 1102)]
    #[display("channel_info({0})", alt = "{0:#}")]
    #[from]
    SwapInfo(SwapInfo),

    #[lnp_api(type = 1103)]
    #[display("peer_list({0})", alt = "{0:#}")]
    #[from]
    PeerList(List<NodeAddr>),

    #[lnp_api(type = 1104)]
    #[display("channel_list({0})", alt = "{0:#}")]
    #[from]
    SwapList(List<SwapId>),

    #[lnp_api(type = 1203)]
    #[display("channel_funding({0})", alt = "{0:#}")]
    #[from]
    SwapFunding(PubkeyScript),
}

impl rpc_connection::Request for Request {}

#[derive(Clone, PartialEq, Eq, Debug, Display, StrictEncode, StrictDecode)]
#[strict_encoding_crate(lnpbp::strict_encoding)]
#[display("{peerd}, ...")]
pub struct CreateSwap {
    pub swap_req: message::OpenChannel,
    pub peerd: ServiceId,
    pub report_to: Option<ServiceId>,
}

#[cfg_attr(feature = "serde", serde_as)]
#[derive(Clone, PartialEq, Eq, Debug, Display, StrictEncode, StrictDecode)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(crate = "serde_crate")
)]
#[strict_encoding_crate(lnpbp::strict_encoding)]
#[display(NodeInfo::to_yaml_string)]
pub struct NodeInfo {
    pub node_id: secp256k1::PublicKey,
    pub listens: Vec<RemoteSocketAddr>,
    #[serde_as(as = "DurationSeconds")]
    pub uptime: Duration,
    pub since: u64,
    #[serde_as(as = "Vec<DisplayFromStr>")]
    pub peers: Vec<NodeAddr>,
    #[serde_as(as = "Vec<DisplayFromStr>")]
    pub swaps: Vec<SwapId>,
}

#[cfg_attr(feature = "serde", serde_as)]
#[derive(Clone, PartialEq, Eq, Debug, Display, StrictEncode, StrictDecode)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(crate = "serde_crate")
)]
#[strict_encoding_crate(lnpbp::strict_encoding)]
#[display(PeerInfo::to_yaml_string)]
pub struct PeerInfo {
    pub local_id: secp256k1::PublicKey,
    pub remote_id: Vec<secp256k1::PublicKey>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub local_socket: Option<InetSocketAddr>,
    #[serde_as(as = "Vec<DisplayFromStr>")]
    pub remote_socket: Vec<InetSocketAddr>,
    #[serde_as(as = "DurationSeconds")]
    pub uptime: Duration,
    pub since: u64,
    pub messages_sent: usize,
    pub messages_received: usize,
    pub connected: bool,
    pub awaits_pong: bool,
}

pub type RemotePeerMap<T> = BTreeMap<NodeAddr, T>;

//#[serde_as]
#[cfg_attr(feature = "serde", serde_as)]
#[derive(Clone, PartialEq, Eq, Debug, Display, StrictEncode, StrictDecode)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(crate = "serde_crate")
)]
#[strict_encoding_crate(lnpbp::strict_encoding)]
#[display(SwapInfo::to_yaml_string)]
pub struct SwapInfo {
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub channel_id: Option<SwapId>,
    #[serde_as(as = "DisplayFromStr")]
    pub temporary_channel_id: TempSwapId,
    pub state: Lifecycle,
    pub local_capacity: u64,
    #[serde_as(as = "BTreeMap<DisplayFromStr, Same>")]
    pub remote_capacities: RemotePeerMap<u64>,
    #[serde_as(as = "Vec<DisplayFromStr>")]
    pub assets: Vec<AssetId>,
    #[serde_as(as = "BTreeMap<DisplayFromStr, Same>")]
    pub local_balances: AssetsBalance,
    #[serde_as(
        as = "BTreeMap<DisplayFromStr, BTreeMap<DisplayFromStr, Same>>"
    )]
    pub remote_balances: RemotePeerMap<AssetsBalance>,
    pub funding_outpoint: OutPoint,
    #[serde_as(as = "Vec<DisplayFromStr>")]
    pub remote_peers: Vec<NodeAddr>,
    #[serde_as(as = "DurationSeconds")]
    pub uptime: Duration,
    pub since: u64,
    pub commitment_updates: u64,
    pub total_payments: u64,
    pub pending_payments: u16,
    pub is_originator: bool,
    pub params: payment::channel::Params,
    pub local_keys: payment::channel::Keyset,
    #[serde_as(as = "BTreeMap<DisplayFromStr, Same>")]
    pub remote_keys: BTreeMap<NodeAddr, payment::channel::Keyset>,
}

#[cfg(feature = "serde")]
impl ToYamlString for NodeInfo {}
#[cfg(feature = "serde")]
impl ToYamlString for PeerInfo {}
#[cfg(feature = "serde")]
impl ToYamlString for SwapInfo {}

#[derive(
    Wrapper, Clone, PartialEq, Eq, Debug, From, StrictEncode, StrictDecode,
)]
#[strict_encoding_crate(lnpbp::strict_encoding)]
#[wrapper(IndexRange)]
pub struct List<T>(Vec<T>)
where
    T: Clone + PartialEq + Eq + Debug + Display + StrictEncode + StrictDecode;

#[cfg(feature = "serde")]
impl<'a, T> Display for List<T>
where
    T: Clone
        + PartialEq
        + Eq
        + Debug
        + Display
        + serde::Serialize
        + StrictEncode
        + StrictDecode,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(
            &serde_yaml::to_string(self)
                .expect("internal YAML serialization error"),
        )
    }
}

impl<T> FromIterator<T> for List<T>
where
    T: Clone
        + PartialEq
        + Eq
        + Debug
        + Display
        + serde::Serialize
        + StrictEncode
        + StrictDecode,
{
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self::from_inner(iter.into_iter().collect())
    }
}

#[cfg(feature = "serde")]
impl<T> serde::Serialize for List<T>
where
    T: Clone
        + PartialEq
        + Eq
        + Debug
        + Display
        + serde::Serialize
        + StrictEncode
        + StrictDecode,
{
    fn serialize<S>(
        &self,
        serializer: S,
    ) -> Result<<S as serde::Serializer>::Ok, <S as serde::Serializer>::Error>
    where
        S: serde::Serializer,
    {
        self.as_inner().serialize(serializer)
    }
}

#[derive(
    Wrapper,
    Clone,
    PartialEq,
    Eq,
    Debug,
    From,
    Default,
    StrictEncode,
    StrictDecode,
)]
#[strict_encoding_crate(lnpbp::strict_encoding)]
pub struct OptionDetails(pub Option<String>);

impl Display for OptionDetails {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.as_inner() {
            None => Ok(()),
            Some(msg) => f.write_str(&msg),
        }
    }
}

impl OptionDetails {
    pub fn with(s: impl ToString) -> Self {
        Self(Some(s.to_string()))
    }

    pub fn new() -> Self {
        Self(None)
    }
}

impl From<crate::Error> for Request {
    fn from(err: crate::Error) -> Self {
        Request::Failure(Failure::from(err))
    }
}

pub trait IntoProgressOrFalure {
    fn into_progress_or_failure(self) -> Request;
}
pub trait IntoSuccessOrFalure {
    fn into_success_or_failure(self) -> Request;
}

impl IntoProgressOrFalure for Result<String, crate::Error> {
    fn into_progress_or_failure(self) -> Request {
        match self {
            Ok(val) => Request::Progress(val),
            Err(err) => Request::from(err),
        }
    }
}

impl IntoSuccessOrFalure for Result<String, crate::Error> {
    fn into_success_or_failure(self) -> Request {
        match self {
            Ok(val) => Request::Success(OptionDetails::with(val)),
            Err(err) => Request::from(err),
        }
    }
}

impl IntoSuccessOrFalure for Result<(), crate::Error> {
    fn into_success_or_failure(self) -> Request {
        match self {
            Ok(_) => Request::Success(OptionDetails::new()),
            Err(err) => Request::from(err),
        }
    }
}
