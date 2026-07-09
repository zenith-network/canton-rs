pub struct PageToken {
    inner: Vec<u8>,
}

impl PageToken {
    pub fn new(inner: Vec<u8>) -> Self {
        Self { inner }
    }
}

impl AsRef<[u8]> for PageToken {
    fn as_ref(&self) -> &[u8] {
        self.inner.as_ref()
    }
}

impl From<PageToken> for Vec<u8> {
    fn from(value: PageToken) -> Self {
        value.inner
    }
}

pub struct Page<T> {
    pub items: Vec<T>,
    pub lowest_page_offset_exclusive: i64,
    pub highest_page_offset_inclusive: i64,
    pub next_page_token: Option<PageToken>,
}
