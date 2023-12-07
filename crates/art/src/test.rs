#[test]
fn test_string() {
    let mut tree = crate::Art::<str, usize>::new();
    tree.display();
    tree.insert("hello world", 1);
    dbg!(&tree);
    tree.display();
    assert_eq!(tree.get("hello world").copied(), Some(1));
    tree.display();
    tree.insert("hello moon ", 2);
    dbg!(&tree);
    tree.display();
    assert_eq!(tree.get("hello world").copied(), Some(1));
    assert_eq!(tree.get("hello moon ").copied(), Some(2));

    tree.insert("h", 3);
    dbg!(&tree);
    tree.display();
    assert_eq!(tree.get("hello world").copied(), Some(1));
    assert_eq!(tree.get("hello moon ").copied(), Some(2));
    assert_eq!(tree.get("h").copied(), Some(3));

    tree.insert("hello foo", 4);
    dbg!(&tree);
    tree.display();
    assert_eq!(tree.get("hello world").copied(), Some(1));
    assert_eq!(tree.get("hello moon ").copied(), Some(2));
    assert_eq!(tree.get("h").copied(), Some(3));
    assert_eq!(tree.get("hello foo").copied(), Some(4));

    tree.insert("hello boo", 5);
    dbg!(&tree);
    tree.display();
    assert_eq!(tree.get("hello world").copied(), Some(1));
    assert_eq!(tree.get("hello moon ").copied(), Some(2));
    assert_eq!(tree.get("h").copied(), Some(3));
    assert_eq!(tree.get("hello foo").copied(), Some(4));
    assert_eq!(tree.get("hello boo").copied(), Some(5));

    tree.insert("hello voo", 6);
    dbg!(&tree);
    tree.display();
    assert_eq!(tree.get("hello world").copied(), Some(1));
    assert_eq!(tree.get("hello moon ").copied(), Some(2));
    assert_eq!(tree.get("h").copied(), Some(3));
    assert_eq!(tree.get("hello foo").copied(), Some(4));
    assert_eq!(tree.get("hello boo").copied(), Some(5));
    assert_eq!(tree.get("hello voo").copied(), Some(6));

    tree.insert("hello voa", 7);
    dbg!(&tree);
    tree.display();
    assert_eq!(tree.get("hello world").copied(), Some(1));
    assert_eq!(tree.get("hello moon ").copied(), Some(2));
    assert_eq!(tree.get("h").copied(), Some(3));
    assert_eq!(tree.get("hello foo").copied(), Some(4));
    assert_eq!(tree.get("hello boo").copied(), Some(5));
    assert_eq!(tree.get("hello voo").copied(), Some(6));
    assert_eq!(tree.get("hello voa").copied(), Some(7));

    tree.insert(
        "hello a very very long prefix that doesn't fit into an inlined buffer.",
        8,
    );
    dbg!(&tree);
    tree.display();
    assert_eq!(tree.get("hello world").copied(), Some(1));
    assert_eq!(tree.get("hello moon ").copied(), Some(2));
    assert_eq!(tree.get("h").copied(), Some(3));
    assert_eq!(tree.get("hello foo").copied(), Some(4));
    assert_eq!(tree.get("hello boo").copied(), Some(5));
    assert_eq!(tree.get("hello voo").copied(), Some(6));
    assert_eq!(tree.get("hello voa").copied(), Some(7));
    assert_eq!(
        tree.get("hello a very very long prefix that doesn't fit into an inlined buffer.")
            .copied(),
        Some(8)
    );

    tree.insert("hello world\0 null byte", 9);
    tree.display();
    assert_eq!(tree.get("hello world").copied(), Some(1));
    assert_eq!(tree.get("hello moon ").copied(), Some(2));
    assert_eq!(tree.get("h").copied(), Some(3));
    assert_eq!(tree.get("hello foo").copied(), Some(4));
    assert_eq!(tree.get("hello boo").copied(), Some(5));
    assert_eq!(tree.get("hello voo").copied(), Some(6));
    assert_eq!(tree.get("hello voa").copied(), Some(7));
    assert_eq!(
        tree.get("hello a very very long prefix that doesn't fit into an inlined buffer.")
            .copied(),
        Some(8)
    );
    assert_eq!(tree.get("hello world\0 null byte").copied(), Some(9));
}

const XOR_SHIFT_INIT: u64 = 384931938475643;

fn rol64(x: u64, by: u64) -> u64 {
    (x << by) | (x >> (64 - by))
}

struct XorState([u64; 4]);

impl XorState {
    pub fn new() -> Self {
        XorState([39394585, 928328384, 3918384, 1294058])
    }
}

fn xorshift(s: &mut XorState) -> u64 {
    let res = rol64(s.0[1].wrapping_mul(5), 7).wrapping_mul(9);
    let t = s.0[1] << 17;
    s.0[2] ^= s.0[0];
    s.0[3] ^= s.0[1];
    s.0[1] ^= s.0[2];
    s.0[0] ^= s.0[3];

    s.0[2] ^= t;
    s.0[3] = rol64(s.0[3], 45);

    res
}

#[test]
fn test_u64() {
    let mut tree = crate::Art::<u64, u64>::new();
    tree.display();
    println!();
    tree.insert(&0, 0);
    tree.display();
    println!();
    tree.insert(&(1 << 8), 1);
    tree.display();
    println!();
    tree.insert(&(1 << 16), 2);
    tree.display();
    println!();

    assert_eq!(tree.get(&0).copied(), Some(0));
    assert_eq!(tree.get(&(1 << 8)).copied(), Some(1));
    assert_eq!(tree.get(&(1 << 16)).copied(), Some(2));
}

#[test]
fn random_test_u64() {
    let mut tree = crate::Art::<u64, u64>::new();
    let mut state = XorState::new();
    let mut pairs = Vec::new();

    for _ in 0..100_000 {
        let k = xorshift(&mut state);
        let v = xorshift(&mut state);
        tree.insert(&k, v);
        pairs.push((k, v))
    }

    for (k, v) in pairs.iter().copied() {
        assert_eq!(tree.get(&k).copied(), Some(v))
    }
}
