use aart::Aart;

fn main() {
    let mut tree = Aart::<u64, _>::new();
    for key in 0..1000000 {
        tree.insert(&key, key);
    }
}
