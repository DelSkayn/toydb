fn main() {
    let mut tree = art::Art::<str, usize>::new();
    tree.print();
    tree.insert("hello world", 1);

    tree.print();
    assert_eq!(tree.get("hello world").copied(), Some(1));
    tree.print();
    tree.insert("hello moon ", 2);

    tree.print();
    assert_eq!(tree.get("hello world").copied(), Some(1));
    assert_eq!(tree.get("hello moon ").copied(), Some(2));

    tree.insert("h", 3);

    tree.print();
    assert_eq!(tree.get("hello world").copied(), Some(1));
    assert_eq!(tree.get("hello moon ").copied(), Some(2));
    assert_eq!(tree.get("h").copied(), Some(3));

    tree.insert("hello foo", 4);

    tree.print();
    assert_eq!(tree.get("hello world").copied(), Some(1));
    assert_eq!(tree.get("hello moon ").copied(), Some(2));
    assert_eq!(tree.get("h").copied(), Some(3));
    assert_eq!(tree.get("hello foo").copied(), Some(4));

    tree.insert("hello boo", 5);

    tree.print();
    assert_eq!(tree.get("hello world").copied(), Some(1));
    assert_eq!(tree.get("hello moon ").copied(), Some(2));
    assert_eq!(tree.get("h").copied(), Some(3));
    assert_eq!(tree.get("hello foo").copied(), Some(4));
    assert_eq!(tree.get("hello boo").copied(), Some(5));

    tree.insert("hello voo", 6);

    tree.print();
    assert_eq!(tree.get("hello world").copied(), Some(1));
    assert_eq!(tree.get("hello moon ").copied(), Some(2));
    assert_eq!(tree.get("h").copied(), Some(3));
    assert_eq!(tree.get("hello foo").copied(), Some(4));
    assert_eq!(tree.get("hello boo").copied(), Some(5));
    assert_eq!(tree.get("hello voo").copied(), Some(6));

    tree.insert("hello voa", 7);

    tree.print();
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

    tree.print();
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
    tree.print();
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

    let mut iter = tree.iter();
    while let Some((k, v)) = iter.next() {
        dbg!(k, v);
    }
}
