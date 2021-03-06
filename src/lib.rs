use std::{
  collections::BTreeMap,
  io::{Read, Seek, SeekFrom, Write},
  convert::TryFrom,
};

use byteordered::{Endianness, Endian};

mod counter;
mod traits;
pub mod builder;
pub mod error;
pub mod section;
pub mod updater;

use self::{
  counter::Counter,
  error::{Error, Result},
  section::{
    *,
    lbl1::{Group, Label},
  },
  traits::{CalculatesSize, Updates},
  updater::Updater,
};

const HEADER_MAGIC: [u8; 8] = *b"MsgStdBn";
// const LABEL_HASH_MAGIC: u16 = 0x492;
// const LABEL_MAX_LEN: u8 = 64;
// const BYTE_ORDER_OFFSET: u8 = 0x8;
// const HEADER_SIZE: u8 = 0x20;
const PADDING_LENGTH: usize = 16;

#[derive(Debug, Clone, PartialEq)]
pub enum SectionTag {
  Lbl1,
  Nli1,
  Ato1,
  Atr1,
  Tsy1,
  Txt2,
}

#[derive(Debug, Clone)]
pub struct Msbt {
  pub(crate) header: Header,
  pub(crate) section_order: Vec<SectionTag>,
  pub(crate) lbl1: Option<Lbl1>,
  pub(crate) nli1: Option<Nli1>,
  pub(crate) ato1: Option<Ato1>,
  pub(crate) atr1: Option<Atr1>,
  pub(crate) tsy1: Option<Tsy1>,
  pub(crate) txt2: Option<Txt2>,
  pub(crate) pad_byte: u8,
}

impl Msbt {
  pub fn from_reader<R: Read + Seek>(reader: R) -> Result<Self> {
    MsbtReader::new(reader).and_then(|m| Ok(m.msbt))
  }

  pub fn write_to<W: Write>(&self, writer: W) -> Result<()> {
    let mut writer = MsbtWriter::new(self, writer);
    writer.write_header()?;
    for tag in &self.section_order {
      match *tag {
        SectionTag::Lbl1 => writer.write_lbl1()?,
        SectionTag::Nli1 => writer.write_nli1()?,
        SectionTag::Ato1 => writer.write_ato1()?,
        SectionTag::Atr1 => writer.write_atr1()?,
        SectionTag::Tsy1 => writer.write_tsy1()?,
        SectionTag::Txt2 => writer.write_txt2()?,
      }
    }
    Ok(())
  }

  pub fn header(&self) -> &Header {
    &self.header
  }

  pub fn section_order(&self) -> &[SectionTag] {
    &self.section_order
  }

  pub fn lbl1(&self) -> Option<&Lbl1> {
    self.lbl1.as_ref()
  }

  pub fn lbl1_mut(&mut self) -> Option<Updater<Lbl1>> {
    self.lbl1.as_mut().map(Updater::new)
  }

  pub fn nli1(&self) -> Option<&Nli1> {
    self.nli1.as_ref()
  }

  pub fn nli1_mut(&mut self) -> Option<&mut Nli1> {
    self.nli1.as_mut()
  }

  pub fn ato1(&self) -> Option<&Ato1> {
    self.ato1.as_ref()
  }

  pub fn ato1_mut(&mut self) -> Option<&mut Ato1> {
    self.ato1.as_mut()
  }

  pub fn atr1(&self) -> Option<&Atr1> {
    self.atr1.as_ref()
  }

  pub fn atr1_mut(&mut self) -> Option<&mut Atr1> {
    self.atr1.as_mut()
  }

  pub fn tsy1(&self) -> Option<&Tsy1> {
    self.tsy1.as_ref()
  }

  pub fn tsy1_mut(&mut self) -> Option<&mut Tsy1> {
    self.tsy1.as_mut()
  }

  pub fn txt2(&self) -> Option<&Txt2> {
    self.txt2.as_ref()
  }

  pub fn txt2_mut(&mut self) -> Option<Updater<Txt2>> {
    self.txt2.as_mut().map(Updater::new)
  }

  fn plus_padding(size: usize) -> usize {
    let rem = size % PADDING_LENGTH;
    if rem > 0 {
      size + (PADDING_LENGTH - rem)
    } else {
      size
    }
  }
}

impl CalculatesSize for Msbt {
  fn calc_size(&self) -> usize {
    self.header.calc_file_size()
      + Msbt::plus_padding(self.lbl1.as_ref().map(|x| x.calc_size()).unwrap_or(0))
      + Msbt::plus_padding(self.nli1.as_ref().map(CalculatesSize::calc_size).unwrap_or(0))
      + Msbt::plus_padding(self.ato1.as_ref().map(CalculatesSize::calc_size).unwrap_or(0))
      + Msbt::plus_padding(self.atr1.as_ref().map(CalculatesSize::calc_size).unwrap_or(0))
      + Msbt::plus_padding(self.tsy1.as_ref().map(CalculatesSize::calc_size).unwrap_or(0))
      + Msbt::plus_padding(self.txt2.as_ref().map(CalculatesSize::calc_size).unwrap_or(0))
  }
}

impl Updates for Msbt {
  fn update(&mut self) {
    self.header.section_count = self.section_order.len() as u16;
  }
}

#[derive(Debug)]
pub struct MsbtWriter<'a, W> {
  writer: Counter<W>,
  msbt: &'a Msbt,
}

impl<'a, W: Write> MsbtWriter<'a, W> {
  fn new(msbt: &'a Msbt, writer: W) -> Self {
    MsbtWriter {
      msbt,
      writer: Counter::new(writer),
    }
  }

  fn write_header(&mut self) -> Result<()> {
    self.writer.write_all(&self.msbt.header.magic).map_err(Error::Io)?;
    let endianness = match self.msbt.header.endianness {
      Endianness::Big => [0xFE, 0xFF],
      Endianness::Little => [0xFF, 0xFE],
    };
    self.writer.write_all(&endianness).map_err(Error::Io)?;
    self.msbt.header.endianness.write_u16(&mut self.writer, self.msbt.header._unknown_1).map_err(Error::Io)?;
    let encoding_byte = self.msbt.header.encoding as u8;
    self.writer.write_all(&[encoding_byte, self.msbt.header._unknown_2]).map_err(Error::Io)?;
    self.msbt.header.endianness.write_u16(&mut self.writer, self.msbt.header.section_count).map_err(Error::Io)?;
    self.msbt.header.endianness.write_u16(&mut self.writer, self.msbt.header._unknown_3).map_err(Error::Io)?;
    self.msbt.header.endianness.write_u32(&mut self.writer, self.msbt.calc_size() as u32).map_err(Error::Io)?;
    self.writer.write_all(&self.msbt.header.padding).map_err(Error::Io)
  }

  fn write_section(&mut self, section: &Section) -> Result<()> {
    self.writer.write_all(&section.magic).map_err(Error::Io)?;
    self.msbt.header.endianness.write_u32(&mut self.writer, section.size).map_err(Error::Io)?;
    self.writer.write_all(&section.padding).map_err(Error::Io)
  }

  fn write_group(&mut self, group: &Group) -> Result<()> {
    self.msbt.header.endianness.write_u32(&mut self.writer, group.label_count).map_err(Error::Io)?;
    self.msbt.header.endianness.write_u32(&mut self.writer, group.offset).map_err(Error::Io)
  }

  fn write_lbl1(&mut self) -> Result<()> {
    if let Some(ref lbl1) = self.msbt.lbl1 {
      self.write_section(&lbl1.section)?;
      self.msbt.header.endianness.write_u32(&mut self.writer, lbl1.groups().len() as u32).map_err(Error::Io)?;
      for group in &lbl1.groups {
        self.write_group(group)?;
      }
      let mut sorted_labels: Vec<(usize, &Label)> = lbl1.labels.iter().enumerate().collect();
      sorted_labels.sort_by_key(|(_,l)| l.checksum(lbl1));
      for (i, label) in &sorted_labels {
        self.writer.write_all(&[label.name.len() as u8]).map_err(Error::Io)?;
        self.writer.write_all(label.name.as_bytes()).map_err(Error::Io)?;
        self.msbt.header.endianness.write_u32(&mut self.writer, *i as u32).map_err(Error::Io)?;
      }

      self.write_padding()?;
    }
    Ok(())
  }

  pub fn write_nli1(&mut self) -> Result<()> {
    if let Some(ref nli1) = self.msbt.nli1 {
      self.write_section(&nli1.section)?;

      if nli1.section.size > 0 {
        self.msbt.header.endianness.write_u32(&mut self.writer, nli1.id_count).map_err(Error::Io)?;

        for (&key, &val) in &nli1.global_ids {
          self.msbt.header.endianness.write_u32(&mut self.writer, val).map_err(Error::Io)?;
          self.msbt.header.endianness.write_u32(&mut self.writer, key).map_err(Error::Io)?;
        }
      }

      self.write_padding()?;
    }

    Ok(())
  }

  pub fn write_txt2(&mut self) -> Result<()> {
    if let Some(ref txt2) = self.msbt.txt2 {
      self.write_section(&txt2.section)?;

      // write string count
      let value_count = txt2.values.len() as u32;
      self.msbt.header.endianness.write_u32(&mut self.writer, value_count).map_err(Error::Io)?;

      // write offsets
      let mut total = 0;
      for s in &txt2.values {
        let offset = value_count * 4 + 4 + total;
        total += s.len() as u32;
        self.msbt.header.endianness.write_u32(&mut self.writer, offset).map_err(Error::Io)?;
      }

      // write strings
      for s in &txt2.values {
        let value_bytes = s.iter()
          .flat_map(|vv| vv.to_bytes()).collect::<Vec<u8>>();
        self.writer.write_all(&value_bytes).map_err(Error::Io)?;
      }

      self.write_padding()?;
    }

    Ok(())
  }

  pub fn write_ato1(&mut self) -> Result<()> {
    if let Some(ref ato1) = self.msbt.ato1 {
      self.write_section(&ato1.section)?;
      self.writer.write_all(&ato1._unknown).map_err(Error::Io)?;

      self.write_padding()?;
    }

    Ok(())
  }

  pub fn write_atr1(&mut self) -> Result<()> {
    if let Some(ref atr1) = self.msbt.atr1 {
      self.write_section(&atr1.section)?;
      self.writer.write_all(&atr1._unknown).map_err(Error::Io)?;

      self.write_padding()?;
    }

    Ok(())
  }

  pub fn write_tsy1(&mut self) -> Result<()> {
    if let Some(ref tsy1) = self.msbt.tsy1 {
      self.write_section(&tsy1.section)?;
      self.writer.write_all(&tsy1._unknown).map_err(Error::Io)?;

      self.write_padding()?;
    }

    Ok(())
  }

  fn write_padding(&mut self) -> Result<()> {
    let remainder = self.writer.written() % PADDING_LENGTH;
    if remainder == 0 {
      return Ok(());
    }

    self.writer.write_all(&vec![self.msbt.pad_byte; PADDING_LENGTH - remainder]).map_err(Error::Io)
  }
}

#[derive(Debug)]
pub struct MsbtReader<R> {
  reader: R,
  msbt: Msbt,
}

impl<'a, R: Read + Seek> MsbtReader<R> {
  fn new(mut reader: R) -> Result<Self> {
    let header = Header::from_reader(&mut reader)?;

    let mut msbt = MsbtReader {
      reader,
      msbt: Msbt{
        header,
        lbl1: None,
        nli1: None,
        ato1: None,
        atr1: None,
        tsy1: None,
        txt2: None,
        section_order: Vec::with_capacity(6),
        pad_byte: 0,
      }
    };

    msbt.read_sections()?;

    Ok(msbt)
  }

  fn skip_padding(&mut self) -> Result<()> {
    let pos = self.reader.stream_position().map_err(Error::Io)?;
    let remainder = pos % PADDING_LENGTH as u64;
    if remainder > 0 {
      let mut buf = [0; 1];
      self.reader.read_exact(&mut buf).map_err(Error::Io)?;
      self.reader.seek(SeekFrom::Start(pos + PADDING_LENGTH as u64 - remainder)).map_err(Error::Io)?;
      self.msbt.pad_byte = buf[0];
    }
    Ok(())
  }

  pub fn read_sections(&mut self) -> Result<()> {
    let mut peek = [0; 4];
    loop {
      match self.reader.read_exact(&mut peek) {
        Ok(()) => {},
        Err(ref e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(()),
        Err(e) => return Err(Error::Io(e)),
      }

      self.reader.seek(SeekFrom::Current(-4)).map_err(Error::Io)?;

      match &peek {
        b"LBL1" => {
          self.msbt.lbl1 = Some(self.read_lbl1()?);
          self.msbt.section_order.push(SectionTag::Lbl1);
        },
        b"ATR1" => {
          self.msbt.atr1 = Some(self.read_atr1()?);
          self.msbt.section_order.push(SectionTag::Atr1);
        },
        b"ATO1" => {
          self.msbt.ato1 = Some(self.read_ato1()?);
          self.msbt.section_order.push(SectionTag::Ato1);
        },
        b"TSY1" => {
          self.msbt.tsy1 = Some(self.read_tsy1()?);
          self.msbt.section_order.push(SectionTag::Tsy1);
        },
        b"TXT2" => {
          self.msbt.txt2 = Some(self.read_txt2()?);
          self.msbt.section_order.push(SectionTag::Txt2);
        },
        b"NLI1" => {
          self.msbt.nli1 = Some(self.read_nli1()?);
          self.msbt.section_order.push(SectionTag::Nli1);
        },
        _ => return Err(Error::InvalidSection(peek)),
      }

      self.skip_padding()?;
    }
  }

  pub fn read_lbl1(&mut self) -> Result<Lbl1> {
    let section = self.read_section()?;

    if &section.magic != b"LBL1" {
      return Err(Error::InvalidMagic);
    }

    let group_count = self.msbt.header.endianness.read_u32(&mut self.reader).map_err(Error::Io)?;
    let mut groups = Vec::with_capacity(group_count as usize);
    for _ in 0..group_count {
      groups.push(self.read_group()?);
    }

    let label_count = groups.iter().map(|x| x.label_count as usize).sum();
    let mut labels = vec![Label{name: "".to_string()}; label_count];

    let mut buf = [0; 1];
    for group in groups.iter() {
      for _ in 0..group.label_count {
        self.reader.read_exact(&mut buf).map_err(Error::Io)?;
        let str_len = buf[0] as usize;
        let mut str_buf = vec![0; str_len];
        self.reader.read_exact(&mut str_buf).map_err(Error::Io)?;
        let name = String::from_utf8(str_buf).map_err(Error::InvalidUtf8)?;
        let index = self.msbt.header.endianness.read_u32(&mut self.reader).map_err(Error::Io)?;
        labels[index as usize] = Label{ name };
      }
    }

    let lbl1 = Lbl1 {
      section,
      groups,
      labels,
    };

    Ok(lbl1)
  }

  pub fn read_atr1(&mut self) -> Result<Atr1> {
    let section = self.read_section()?;
    let mut unknown = vec![0; section.size as usize];
    self.reader.read_exact(&mut unknown).map_err(Error::Io)?;

    Ok(Atr1 {
      section,
      _unknown: unknown,
    })
  }

  pub fn read_ato1(&mut self) -> Result<Ato1> {
    let section = self.read_section()?;
    let mut unknown = vec![0; section.size as usize];
    self.reader.read_exact(&mut unknown).map_err(Error::Io)?;

    Ok(Ato1 {
      section,
      _unknown: unknown,
    })
  }

  pub fn read_tsy1(&mut self) -> Result<Tsy1> {
    let section = self.read_section()?;
    let mut unknown = vec![0; section.size as usize];
    self.reader.read_exact(&mut unknown).map_err(Error::Io)?;

    Ok(Tsy1 {
      section,
      _unknown: unknown,
    })
  }

  pub fn read_txt2(&mut self) -> Result<Txt2> {
    let section = self.read_section()?;
    let string_count = self.msbt.header.endianness.read_u32(&mut self.reader).map_err(Error::Io)? as usize;

    let mut offsets = Vec::with_capacity(string_count);
    let mut values = Vec::with_capacity(string_count);

    for _ in 0..string_count {
      offsets.push(self.msbt.header.endianness.read_u32(&mut self.reader).map_err(Error::Io)?);
    }

    for i in 0..string_count {
      let next_str_end = if i == string_count - 1 {
        section.size
      } else {
        offsets[i + 1]
      };
      let str_len = next_str_end - offsets[i];
      let mut str_buf = vec![0; str_len as usize];
      self.reader.read_exact(&mut str_buf).map_err(Error::Io)?;
      values.push(txt2::parse_bytes(&str_buf));
    }

    Ok(Txt2 {
      section,
      values,
    })
  }

  pub fn read_nli1(&mut self) -> Result<Nli1> {
    let section = self.read_section()?;

    let mut map = BTreeMap::default();
    let mut id_count = 0;

    if section.size > 0 {
      id_count = self.msbt.header.endianness.read_u32(&mut self.reader).map_err(Error::Io)?;

      for _ in 0..id_count {
        let val = self.msbt.header.endianness.read_u32(&mut self.reader).map_err(Error::Io)?;
        let key = self.msbt.header.endianness.read_u32(&mut self.reader).map_err(Error::Io)?;
        map.insert(key, val);
      }
    }

    Ok(Nli1 {
      section,
      id_count,
      global_ids: map,
    })
  }

  pub fn read_group(&mut self) -> Result<Group> {
    let label_count = self.msbt.header.endianness.read_u32(&mut self.reader).map_err(Error::Io)?;
    let offset = self.msbt.header.endianness.read_u32(&mut self.reader).map_err(Error::Io)?;

    Ok(Group {
      label_count,
      offset,
    })
  }

  pub fn read_section(&mut self) -> Result<Section> {
    let mut magic = [0; 4];
    let mut padding = [0; 8];

    self.reader.read_exact(&mut magic).map_err(Error::Io)?;
    let size = self.msbt.header.endianness.read_u32(&mut self.reader).map_err(Error::Io)?;
    self.reader.read_exact(&mut padding).map_err(Error::Io)?;

    Ok(Section {
      magic,
      size,
      padding,
    })
  }
}

#[derive(Debug, Clone)]
pub struct Header {
  pub(crate) magic: [u8; 8],
  pub(crate) endianness: Endianness,
  pub(crate) _unknown_1: u16,
  pub(crate) encoding: Encoding,
  pub(crate) _unknown_2: u8,
  pub(crate) section_count: u16,
  pub(crate) _unknown_3: u16,
  pub(crate) padding: [u8; 10],
}

impl Header {
  pub fn from_reader(mut reader: &mut dyn Read) -> Result<Self> {
    let mut buf = [0u8; 10];
    reader.read_exact(&mut buf[..8]).map_err(Error::Io)?;

    let mut magic = [0u8; 8];
    magic.swap_with_slice(&mut buf[..8]);
    if magic != HEADER_MAGIC {
      return Err(Error::InvalidMagic);
    }

    reader.read_exact(&mut buf[..2]).map_err(Error::Io)?;
    let endianness = match buf[..2] {
      [0xFE, 0xFF] => Endianness::Big,
      [0xFF, 0xFE] => Endianness::Little,
      _ => return Err(Error::InvalidBom),
    };

    let unknown_1 = endianness.read_u16(&mut reader).map_err(Error::Io)?;

    reader.read_exact(&mut buf[..1]).map_err(Error::Io)?;
    let encoding = Encoding::try_from(buf[0])
      .map_err(|_| Error::InvalidEncoding(buf[0]))?;

    reader.read_exact(&mut buf[..1]).map_err(Error::Io)?;
    let unknown_2 = buf[0];

    let section_count = endianness.read_u16(&mut reader).map_err(Error::Io)?;
    let unknown_3 = endianness.read_u16(&mut reader).map_err(Error::Io)?;
    let _file_size = endianness.read_u32(&mut reader).map_err(Error::Io)?;

    reader.read_exact(&mut buf[..10]).map_err(Error::Io)?;
    let padding = buf;

    Ok(Header {
      magic,
      endianness,
      encoding,
      section_count,
      padding,
      _unknown_1: unknown_1,
      _unknown_2: unknown_2,
      _unknown_3: unknown_3,
    })
  }

  pub fn magic(&self) -> [u8; 8] {
    self.magic
  }

  pub fn endianness(&self) -> Endianness {
    self.endianness
  }

  pub fn unknown_1(&self) -> u16 {
    self._unknown_1
  }

  pub fn encoding(&self) -> Encoding {
    self.encoding
  }

  pub fn unknown_2(&self) -> u8 {
    self._unknown_2
  }

  pub fn section_count(&self) -> u16 {
    self.section_count
  }

  pub fn unknown_3(&self) -> u16 {
    self._unknown_3
  }

  pub fn padding(&self) -> [u8; 10] {
    self.padding
  }

  pub(crate) fn calc_file_size(&self) -> usize {
    std::mem::size_of_val(&self.magic)
      + std::mem::size_of::<u16>() // endianness
      + std::mem::size_of_val(&self._unknown_1)
      + std::mem::size_of::<u8>() // encoding
      + std::mem::size_of_val(&self._unknown_2)
      + std::mem::size_of_val(&self.section_count)
      + std::mem::size_of_val(&self._unknown_3)
      + std::mem::size_of::<u32>() // file size
      + std::mem::size_of_val(&self.padding)
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
  Utf8 = 0x00,
  Utf16 = 0x01,
}

impl std::convert::TryFrom<u8> for Encoding {
  type Error = ();

  fn try_from(value: u8) -> std::result::Result<Encoding, ()> {
      Ok(match value {
        0x00 => Encoding::Utf8,
        0x01 => Encoding::Utf16,
        _ => return Err(()),
      })
  }
}
