use crate::{
    key::Key,
    raw::{LeafNode, NodePtr, TypedNodePtr},
    Art,
};
use std::marker::PhantomData;

pub struct RawIterator<'a, O, K: Key + ?Sized, V> {
    pub key: Vec<u8>,
    pub ptr: Option<NodePtr<O, K, V>>,
    pub _marker: PhantomData<&'a Art<K, V>>,
}

type Item<'a, O, K, V> = (&'a [u8], TypedNodePtr<O, LeafNode<K, V>>);

impl<O, K: Key + ?Sized, V> RawIterator<'_, O, K, V> {
    pub fn next(&mut self) -> Option<Item<'_, O, K, V>> {
        todo!()
        /*
        unsafe {
            if self.key.is_empty() {
                // startup
                let mut ptr = self.ptr?;
                dbg!(ptr.into_ptr());

                loop {
                    self.key.extend_from_slice(ptr.header().storage.prefix());

                    if ptr.is::<LeafNode<K, V>>() {
                        self.ptr = Some(ptr);
                        return Some((self.key.as_slice(), ptr.into_owned()));
                    }

                    let (key, new_ptr) = ptr.next_node(0).expect("no leaf nodes in branch node");
                    dbg!(key);
                    dbg!(new_ptr.into_ptr());
                    self.key.push(key);
                    ptr = new_ptr;
                }
            }

            let mut ptr = self.ptr?;

            let new_len = self.key.len() - ptr.header().storage.prefix().len();
            self.key.truncate(new_len);

            ptr = ptr.header().parent?;

            loop {
                let last_key = self.key.last_mut().unwrap();

                if *last_key == 255 {
                    self.ptr = ptr.header().parent;
                    let new_len = self.key.len() - ptr.prefix().len() - 1;
                    self.key.truncate(new_len);
                    continue;
                }

                let Some((found, new_ptr)) = ptr.next_node(*last_key + 1) else {
                    self.ptr = ptr.header().parent;
                    let new_len = self.key.len() - ptr.prefix().len() - 1;
                    self.key.truncate(new_len);
                    continue;
                };

                *last_key = found;

                self.key
                    .extend_from_slice(new_ptr.header().storage.prefix());

                ptr = new_ptr;
                break;
            }

            loop {
                if ptr.is::<LeafNode<K, V>>() {
                    self.ptr = Some(ptr);
                    return Some((self.key.as_slice(), ptr.into_owned()));
                }

                let (key, new_ptr) = ptr.next_node(0).expect("no leaf nodes in branch node");

                self.key.push(key);
                self.key
                    .extend_from_slice(new_ptr.header().storage.prefix());

                ptr = new_ptr;
            }
        }
        */
    }
}
