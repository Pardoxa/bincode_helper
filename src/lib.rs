use std::{io::{Write, Read}, marker::PhantomData};
use anyhow::{Result, Context};
use bincode::Options;
use serde::{Serialize, de::DeserializeOwned};


pub struct SerializeAnyhow<W>
{
    buffer1: Vec<u8>,
    buffer2: Vec<u8>,
    writer: W
}

impl<W> SerializeAnyhow<W>
{
    pub fn serialize_something<T>(&mut self, something: &T) -> Result<()>
    where T: Serialize,
        W: Write
    {
        self.buffer1.clear();
        let options = bincode::DefaultOptions::new();
        options.serialize_into(&mut self.buffer1, something)
            .with_context(|| "serialization of T")?;
        let size = self.buffer1.len() as u64;
        self.buffer2.clear();
        
        bincode::serialize_into(&mut self.buffer2, &size)
            .with_context(|| "serialization of len")?;
        self.writer
            .write_all(&self.buffer2)
            .with_context(|| "error during writing of len")?;
        self.writer
            .write_all(&self.buffer1)
            .with_context(|| "error during writing some T")
    }

    pub fn new(writer: W) -> Self
    {
        Self{
            writer,
            buffer1: Vec::with_capacity(10240),
            buffer2: Vec::with_capacity(10240)
        }
    }
}

pub struct DeserializeAnyhow<R>
{
    buffer1: Vec<u8>,
    buffer2: Vec<u8>,
    reader: R
}

impl<R> DeserializeAnyhow<R>
where R: Read
{
    pub fn new(reader: R) -> Self
    {
        Self
        {
            reader,
            buffer1: Vec::new(),
            buffer2: vec![0_u8; 8]
        }
    }

    pub fn deserialize<T>(&mut self) -> Result<T>
    where T: DeserializeOwned
    {
        self.reader.read_exact(&mut self.buffer2)
            .with_context(|| "during reading of len")?;

        let size: u64 = bincode::deserialize(&self.buffer2)
            .with_context(|| "during deserializing len")?;
        let size = size as usize;

        if self.buffer1.len() >= size
        {
            self.buffer1.truncate(size);
        } else {
            let missing = size - self.buffer1.len();
            self.buffer1.extend((0..missing).map(|_| 0));
        }
        
        self.reader.read_exact(&mut self.buffer1)
            .with_context(|| "during reading of T")?;

        let options = bincode::DefaultOptions::new();
        options.deserialize(&self.buffer1)
            .with_context(|| "Deserialization of T did not succeed")
    }

    pub fn create_vec<T>(&mut self, size_limit: Option<usize>) -> Vec<T>
    where T: DeserializeOwned
    {
        let iter = ReadingIter{
            deserializer: self,
            item: PhantomData::<T>
        };
        if let Some(limit) = size_limit
        {
            iter.take(limit)
                .collect()
        } else {
            iter.collect()
        }

    }
}

struct ReadingIter<'a, R, I>
{
    deserializer: &'a mut DeserializeAnyhow<R>,
    item: PhantomData<I>
}

impl<'a, R, I> Iterator for ReadingIter<'a, R, I>
where R: Read,
    I: DeserializeOwned
{
    type Item = I;

    fn next(&mut self) -> Option<Self::Item> {
        self.deserializer.deserialize().ok()
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    use std::fs::File;
    use std::io::{BufWriter, BufReader};
    use serde::Deserialize;

    #[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
    pub struct Testing
    {
        a: u32,
        b: i32
    }

    #[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
    pub struct Testing2
    {
        bla: Vec<String>
    }

    #[test]
    fn seralize_and_deserialize_something() {
        let testa = Testing{a: 12, b: -12};
        let testb = Testing{a: 1, b: -3};

        let v = vec!["hallo ich".to_owned(), "This is end".to_owned(), "ICH WILL WEI?WOIOWOUWGFIUWGFIOEUGFEUIGFUILWEGFBLIEUOFLHILU".to_owned()];

        let testc = Testing2{bla: v};

        let mut encoded: Vec<u8> = bincode::serialize(&testa).unwrap();
        bincode::serialize_into(&mut encoded, &testb).unwrap();

        let test_file = File::create("Test.bincode").unwrap();
        let buf = BufWriter::new(test_file);

        let mut helper = SerializeAnyhow::new(buf);

        helper.serialize_something(&testa).unwrap();
        helper.serialize_something(&testb).unwrap();
        helper.serialize_something(&testc).unwrap();
        drop(helper);
        let file = File::open("Test.bincode").unwrap();
        let reader = BufReader::new(file);
        let mut de_helper = DeserializeAnyhow::new(reader);

        let a: Testing = de_helper.deserialize().unwrap();
        let b: Testing = de_helper.deserialize().unwrap();
        let c: Testing2 = de_helper.deserialize().unwrap();
        
        assert_eq!(a, testa);
        assert_eq!(b, testb);
        assert_eq!(c, testc)
    }
}
