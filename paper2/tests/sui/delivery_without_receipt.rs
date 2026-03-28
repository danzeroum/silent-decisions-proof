// compile-fail test: Persistence Enclosure
// A DeliveryToken cannot be constructed without a valid InclusionReceipt.
// Since InclusionReceipt::new is pub(crate), external code has no path to it.
// Expected: no well-typed expression produces DeliveryToken in external crate.

use btv_transparency::DeliveryToken;

struct FakeVerdict;
impl serde::Serialize for FakeVerdict {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_unit()
    }
}

fn main() {
    // The only constructor is DeliveryToken::seal(verdict, receipt).
    // We have no way to obtain a valid InclusionReceipt — this must NOT compile
    // because InclusionReceipt cannot be constructed outside the crate.
    let receipt = btv_transparency::InclusionReceipt::new(0, todo!(), todo!());
    let _token = DeliveryToken::seal(&FakeVerdict, receipt);
}
