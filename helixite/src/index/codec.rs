pub struct KeyBuilder {
    bytes: Vec<u8>,
}

impl Default for KeyBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyBuilder {
    pub fn new() -> Self {
        Self {
            bytes: Vec::with_capacity(32),
        }
    }

    pub fn u8(mut self, value: u8) -> Self {
        self.bytes.push(value);
        self
    }

    pub fn str(mut self, value: &str) -> Self {
        let len = u32::try_from(value.len()).expect("key string field exceeds u32::MAX");
        self.bytes.extend(len.to_be_bytes());
        self.bytes.extend(value.as_bytes());
        self
    }

    pub fn bytes(mut self, value: &[u8]) -> Self {
        let len = u32::try_from(value.len()).expect("key bytes field exceeds u32::MAX");
        self.bytes.extend(len.to_be_bytes());
        self.bytes.extend(value);
        self
    }

    pub fn u64(mut self, value: u64) -> Self {
        self.bytes.extend(value.to_be_bytes());
        self
    }

    pub fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

pub struct KeyReader<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> KeyReader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, cursor: 0 }
    }

    pub fn u8(&mut self) -> Option<u8> {
        if self.cursor + 1 > self.bytes.len() {
            return None;
        }
        let val = self.bytes[self.cursor];
        self.cursor += 1;
        Some(val)
    }

    pub fn str(&mut self) -> Option<&'a [u8]> {
        let len = usize::try_from(self.u32()?).ok()?;
        if self.cursor + len > self.bytes.len() {
            return None;
        }
        let start = self.cursor;
        self.cursor += len;
        Some(&self.bytes[start..self.cursor])
    }

    pub fn bytes(&mut self) -> Option<&'a [u8]> {
        let len = usize::try_from(self.u32()?).ok()?;
        if self.cursor + len > self.bytes.len() {
            return None;
        }
        let start = self.cursor;
        self.cursor += len;
        Some(&self.bytes[start..self.cursor])
    }

    pub fn u64(&mut self) -> Option<u64> {
        if self.cursor + 8 > self.bytes.len() {
            return None;
        }
        let bytes: [u8; 8] = self.bytes[self.cursor..self.cursor + 8].try_into().ok()?;
        self.cursor += 8;
        Some(u64::from_be_bytes(bytes))
    }

    fn u32(&mut self) -> Option<u32> {
        if self.cursor + 4 > self.bytes.len() {
            return None;
        }
        let bytes: [u8; 4] = self.bytes[self.cursor..self.cursor + 4].try_into().ok()?;
        self.cursor += 4;
        Some(u32::from_be_bytes(bytes))
    }

    pub fn finish(self) -> Option<()> {
        if self.cursor == self.bytes.len() {
            Some(())
        } else {
            None
        }
    }
}
