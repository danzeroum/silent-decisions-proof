// compile-fail test: DeliveryToken cannot be delivered twice (linear consumption).
// deliver() consumes self by value; a second call is a use-after-move.
// Expected: E0382 — use of moved value: `token`

use btv_transparency::DeliveryToken;

fn double_deliver(token: DeliveryToken) {
    let _p1 = token.deliver(); // consumes token
    let _p2 = token.deliver(); // E0382: use of moved value
}

fn main() {}
