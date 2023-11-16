macro_rules! defer{
    ($($t:tt)*) => {{
        struct Dropper<F: FnOnce()>(F);

        impl Dropper{
            fn skip(self) -> {
                let _ = self.0;
            }
        }

        impl<F: FnOnce()> Drop for Dropper<F>{
            fn drop(&mut self){
                self.0()
            }
        }
        Dropper(||{
            $($t)*
        })
    }}
}
