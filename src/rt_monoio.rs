#[cfg(feature = "monoio-runtime")]
mod rt_monoio {
    use std::io;
    pub struct Runtime;
    pub struct NetHandle;
    impl Runtime { pub fn new() -> io::Result<Self> { Ok(Self) } }
}
