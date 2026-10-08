//! A question judged by named facts is cut to those facts before its answer is read into the law's value
//! domain: a member no decision reads (the fee history's fractional `gasUsedRatio`) never makes the whole answer
//! unreadable. In-process nodes only; no network.

use zikaron::json::Value;
use zikaron_anchor::judge::{self, Missing};
use zikaron_anchor::rpc::{Endpoint, Trouble};
use zikaron_anchor::send;
use zikaron_anchor::wire::{self, W};

fn w(s: &str) -> W {
    wire::parse(s.as_bytes()).expect("a wire value")
}

/// A fee history as nodes give it: the paid priority fees, and each block's fractional `gasUsedRatio`.
const FEE_HISTORY: &str = "{\"oldestBlock\":\"0x3d\",\"baseFeePerGas\":[\"0x3b9aca00\",\"0x3b9aca00\",\"0x3b9aca00\",\"0x3b9aca00\"],\"gasUsedRatio\":[0.5,0.123456789,0.0],\"reward\":[[\"0x5f5e100\"],[\"0x5f5e100\"],[\"0x5f5e100\"]]}";

/// A node in this process answering every question of a fee reading, the fee history with fractions.
struct Node;

impl Endpoint for Node {
    fn call(&mut self, method: &str, _params: &Value) -> Result<W, Trouble> {
        Ok(w(match method {
            "eth_blockNumber" => "\"0x40\"",
            "eth_getBlockByNumber" => "{\"baseFeePerGas\":\"0x3b9aca00\",\"number\":\"0x40\",\"gasUsedRatio\":0.25,\"timestamp\":\"0x64\"}",
            "eth_feeHistory" => FEE_HISTORY,
            _ => "null",
        }))
    }
    fn name(&self) -> String {
        "http://127.0.0.1:1".into()
    }
}

/// The fee history as a whole is outside the law's value domain (a fraction). Judged through the judging table
/// it is read as the `reward` fact alone, from both nodes, with no node passed over as unreadable; the one-node
/// reading (`read_as_decided`) reads it too, and the fee reading's tip is read, not `None` (`send::read_fees`
/// over a node answering fractions gives the chain's fees, not the fallback). A question judged on the whole
/// answer stays whole: the same fraction there is unreadable, by name.
#[test]
fn a_member_no_decision_reads_does_not_make_the_answer_unreadable() {
    let params = send::tip_params(0x40);
    let history = w(FEE_HISTORY);
    // The precondition: whole, the answer does not read into the law's values.
    assert!(wire::to_core(&history).is_none(), "a fraction is outside the value domain");
    // Named facts: the fee history is judged by `reward` alone.
    assert_eq!(judge::facts_of("eth_feeHistory", &params), Some(&judge::FEE_HISTORY_FACTS[..]));
    let read = judge::read_as_decided("eth_feeHistory", &params, &history).expect("read as its decision reads it");
    assert!(read.member("reward").is_some());
    assert!(read.member("gasUsedRatio").is_none() && read.member("oldestBlock").is_none(), "only the named members: {read:?}");
    assert_eq!(send::tip_of(&read), Some(100_000_000), "the tip is read");
    // Through the judging table, two nodes alike: a reading from both, nobody passed over as unreadable.
    let answers = vec![("http://a".to_string(), Ok(history.clone())), ("http://b".to_string(), Ok(w(FEE_HISTORY)))];
    let j = judge::judge("eth_feeHistory", &params, answers).expect("the fee history is read");
    assert_eq!((j.sources.len(), j.single_source), (2, false));
    assert!(j.missing.is_empty(), "no node passed over: {:?}", j.missing);
    assert!(j.value.member("gasUsedRatio").is_none());
    assert_eq!(send::tip_of(&j.value), Some(100_000_000));
    // Two nodes whose fractions differ do not differ: only the named facts are compared.
    let other = w(&FEE_HISTORY.replace("0.123456789", "0.9"));
    let j = judge::judge("eth_feeHistory", &params, vec![("http://a".to_string(), Ok(history.clone())), ("http://b".to_string(), Ok(other))]).expect("alike in the named facts");
    assert_eq!(j.sources.len(), 2);
    // A pinned block carrying a fraction beside its base fee: the base fee is read.
    let pinned = Value::Arr(vec![Value::Str("0x40".into()), Value::Bool(false)]);
    let block = w("{\"baseFeePerGas\":\"0x3b9aca00\",\"gasUsedRatio\":0.25}");
    let j = judge::judge("eth_getBlockByNumber", &pinned, vec![("http://a".to_string(), Ok(block))]).expect("the block is read");
    assert_eq!(send::base_fee_of(&j.value), Some(1_000_000_000));
    // The fee reading over a node answering fractions: the tip is read, the fees are the chain's.
    let fees = send::read_fees(&mut Node);
    assert!(fees.from_chain, "{fees:?}");
    assert_eq!(fees, send::Fees::of(Some(1_000_000_000), Some(100_000_000)), "the tip read, not its ceiling");
    // A question judged whole stays whole: the fraction there makes it unreadable, said by its text.
    let j = judge::judge("eth_getBalance", &Value::Arr(vec![]), vec![("http://a".to_string(), Ok(w("0.5")))]);
    match j {
        Err(judge::NoReading::NoneAnswered(m)) => assert!(matches!(m.as_slice(), [(_, Missing::Unreadable(_))]), "{m:?}"),
        other => panic!("a whole answer outside the domain is unreadable: {other:?}"),
    }
}
